//! Formateurs de sortie CLI.

use crate::application::docs::DocsResult;
use crate::application::scan::ScanResult;
use crate::application::setup::SetupResult;
use crate::application::status::StatusResult;
use crate::domain::change::{Change, ChangeKind};
use crate::domain::relation::Confidence;
use crate::domain::source::SourceElementKind;
use serde_json::json;

/// Émet l'avertissement « 0 fichier analysé » sur `stderr` (cli.md §3).
///
/// Cette fonction est le **seul** point d'émission de cet avertissement.
/// Elle doit être appelée après toute analyse qui produit un `SourceModel` vide.
/// L'avertissement est émis même avec `--json` et n'apparaît jamais dans le JSON.
pub fn emit_empty_analysis_warning() {
    eprintln!("Warning: 0 source file analyzed.");
    eprintln!(
        "Hint: the project may use a language that is not supported yet (only Python is analyzed)."
    );
}

/// Version du contrat JSON (cli.md §11, décision 15).
///
/// Incrémentée **uniquement** en cas de rupture (renommage, suppression ou changement
/// de type d'un champ existant). Un ajout de champ ne change pas la version.
pub const SCHEMA_VERSION: u32 = 1;

pub fn format_setup_text(result: &SetupResult) -> String {
    let status_msg = if result.overwritten {
        "Configuration overwritten."
    } else {
        "Configuration created."
    };

    format!(
        "Project docs:\n  {}\n\nProject:\n  {}\n\n{}",
        result.project_docs.display(),
        result.project.display(),
        status_msg
    )
}

pub fn format_setup_json(result: &SetupResult) -> String {
    let config_path = result.project_docs.join(".agentic-doc").join("config.json");
    let json_val = json!({
        "command": "setup",
        "schema_version": SCHEMA_VERSION,
        "project_docs": result.project_docs.to_string_lossy(),
        "project": result.project.to_string_lossy(),
        "config": config_path.to_string_lossy(),
        "overwritten": result.overwritten
    });
    serde_json::to_string_pretty(&json_val).unwrap()
}

pub fn format_scan_text(result: &ScanResult) -> String {
    format!(
        "Project scanned.\n\nProject:\n  {}\n\nFiles analyzed:\n  {}\n\nElements found:\n  {}\n\nSnapshot:\n  .agentic-doc/snapshots/{}.json",
        result.project.root.display(),
        result.files_count,
        result.elements_count,
        result.snapshot.id.0
    )
}

pub fn format_scan_json(result: &ScanResult) -> String {
    let json_val = json!({
        "command": "scan",
        "schema_version": SCHEMA_VERSION,
        "project": result.project.root.to_string_lossy(),
        "snapshot_id": result.snapshot.id.0,
        "files": result.files_count,
        "elements": result.elements_count
    });
    serde_json::to_string_pretty(&json_val).unwrap()
}

/// Fichiers rapportés « en bloc » : ajoutés ou supprimés en entier.
///
/// Les éléments d'un tel fichier ne sont pas listés individuellement dans la sortie
/// texte : ils sont forcément tous nouveaux, ou forcément tous disparus. Les énumérer
/// noierait la sortie sans rien apprendre (cli.md §6). Le JSON, lui, reste complet.
fn wholesale_files(changes: &[Change]) -> std::collections::BTreeSet<&str> {
    changes
        .iter()
        .filter(|c| {
            c.element_kind == SourceElementKind::File
                && matches!(c.kind, ChangeKind::Added | ChangeKind::Removed)
        })
        .map(|c| c.file.as_str())
        .collect()
}

/// Nombre de changements d'éléments masqués par la règle d'affichage (cli.md §6).
fn hidden_element_changes(changes: &[Change]) -> usize {
    let wholesale = wholesale_files(changes);
    changes
        .iter()
        .filter(|c| {
            c.element_kind != SourceElementKind::File && wholesale.contains(c.file.as_str())
        })
        .count()
}

/// Suffixe du résumé indiquant les changements d'éléments non listés.
///
/// Le résumé continue de compter tous les changements détectés, comme le JSON : ce
/// suffixe rend le décompte explicable (cli.md §6).
fn hidden_suffix(changes: &[Change]) -> String {
    match hidden_element_changes(changes) {
        0 => String::new(),
        n => format!(" ({n} inside added or removed files)"),
    }
}

pub fn format_status_text(result: &StatusResult) -> String {
    // Premier run : aucun snapshot précédent
    if result.previous_snapshot.is_none() {
        if result.changeset.changes.is_empty() {
            // Premier run sans aucun fichier analysé — pas de changements à lister
            return "Project status\n\nFirst analysis — no snapshot recorded yet.\nAll elements are reported as added.".to_string();
        }

        let mut out = String::from(
            "Project status\n\nFirst analysis — no snapshot recorded yet.\nAll elements are reported as added.\n\nAdded:\n",
        );

        let wholesale = wholesale_files(&result.changeset.changes);

        let mut added_files = std::collections::BTreeSet::new();
        for c in &result.changeset.changes {
            if c.element_kind == SourceElementKind::File {
                added_files.insert(c.file.as_str());
            }
        }

        for file in &added_files {
            out.push_str(&format!("  {}\n\n", file));
        }

        // Éléments dont le fichier n'est pas rapporté en bloc : aucun au premier run,
        // mais la règle reste valable si un analyzer émet des éléments hors fichier.
        for c in &result.changeset.changes {
            if c.element_kind != SourceElementKind::File && !wholesale.contains(c.file.as_str()) {
                out.push_str(&format!("  {}\n    added\n\n", c.element_id.0));
            }
        }

        let files_added = added_files.len();
        let total_changes = result.changeset.changes.len();
        let file_str = if files_added == 1 { "file" } else { "files" };
        let change_str = if total_changes == 1 {
            "change"
        } else {
            "changes"
        };

        out.push_str(&format!(
            "Summary:\n  {} {} added, 0 unchanged files, {} {}{}",
            files_added,
            file_str,
            total_changes,
            change_str,
            hidden_suffix(&result.changeset.changes)
        ));

        return out;
    }

    // Run normal : snapshot précédent présent
    if result.changeset.changes.is_empty() {
        return "Project status\n\nNo changes since the last snapshot.".to_string();
    }

    let mut out = String::from("Project status\n\nChanged:\n");

    let wholesale = wholesale_files(&result.changeset.changes);

    let mut affected_files = std::collections::HashSet::new();
    for c in &result.changeset.changes {
        affected_files.insert(&c.file);
        if c.element_kind == SourceElementKind::File {
            out.push_str(&format!("  {}\n\n", c.file));
        } else if !wholesale.contains(c.file.as_str()) {
            let action = match c.kind {
                ChangeKind::Added => "added",
                ChangeKind::Removed => "removed",
                ChangeKind::Modified => "modified",
            };
            out.push_str(&format!("  {}\n    {}\n\n", c.element_id.0, action));
        }
    }

    out.push_str("Unchanged:\n");
    for u in &result.changeset.unchanged_files {
        out.push_str(&format!("  {}\n", u));
    }

    let changed_files_count = affected_files.len();
    let unchanged_files_count = result.changeset.unchanged_files.len();
    let total_changes = result.changeset.changes.len();

    let file_str = if changed_files_count == 1 {
        "file"
    } else {
        "files"
    };
    let un_file_str = if unchanged_files_count == 1 {
        "file"
    } else {
        "files"
    };
    let change_str = if total_changes == 1 {
        "change"
    } else {
        "changes"
    };

    out.push_str(&format!(
        "\nSummary:\n  {} changed {}, {} unchanged {}, {} {}{}",
        changed_files_count,
        file_str,
        unchanged_files_count,
        un_file_str,
        total_changes,
        change_str,
        hidden_suffix(&result.changeset.changes)
    ));

    out
}

pub fn format_status_json(result: &StatusResult) -> String {
    let mut json_changes = Vec::new();
    let mut affected_files = std::collections::HashSet::new();

    for c in &result.changeset.changes {
        affected_files.insert(&c.file);
        if c.element_kind == SourceElementKind::File {
            let t = match c.kind {
                ChangeKind::Added => "file_added",
                ChangeKind::Removed => "file_removed",
                ChangeKind::Modified => "file_modified",
            };
            json_changes.push(json!({
                "type": t,
                "file": c.file,
            }));
        } else {
            let t = match c.kind {
                ChangeKind::Added => "element_added",
                ChangeKind::Removed => "element_removed",
                ChangeKind::Modified => "element_modified",
            };
            let kind_str = match c.element_kind {
                SourceElementKind::File => "file",
                SourceElementKind::Class => "class",
                SourceElementKind::Function => "function",
                SourceElementKind::Method => "method",
            };
            json_changes.push(json!({
                "type": t,
                "file": c.file,
                "element": c.element_id.0,
                "kind": kind_str,
            }));
        }
    }

    // `previous_snapshot` vaut null au premier run
    let prev_snap_val = match &result.previous_snapshot {
        Some(snap) => serde_json::Value::String(snap.id.0.clone()),
        None => serde_json::Value::Null,
    };

    let json_val = json!({
        "command": "status",
        "schema_version": SCHEMA_VERSION,
        "project": result.project.root.to_string_lossy(),
        "previous_snapshot": prev_snap_val,
        "current_snapshot": serde_json::Value::Null,
        "changes": json_changes,
        "summary": {
            "changed_files": affected_files.len(),
            "unchanged_files": result.changeset.unchanged_files.len(),
            "changes": result.changeset.changes.len()
        }
    });

    serde_json::to_string_pretty(&json_val).unwrap()
}

use crate::domain::coverage::CoverageEntry;

pub fn format_docs_text(result: &DocsResult) -> String {
    match result {
        DocsResult::Impacts {
            impacts,
            changeset,
            warnings,
            ..
        } => {
            if impacts.is_empty() {
                let change_count = changeset.changes.len();

                // Aucun changement : la question des relations ne se pose pas (cli.md §7).
                if change_count == 0 {
                    return "No documentation impact.\n\nNo changes since the last snapshot."
                        .to_string();
                }

                let change_str = if change_count == 1 {
                    "change"
                } else {
                    "changes"
                };
                return format!(
                    "No documentation impact.\n\n{change_count} {change_str} detected, but no documentation relation was found for them."
                );
            }

            let mut out = String::from("Potentially impacted documentation\n\n");

            for impact in impacts {
                out.push_str(&format!("{}\n\n", impact.document.0));

                out.push_str("Reason:\n");
                for r in &impact.reasons {
                    out.push_str(&format!("  {}\n", r.change.reason));
                }
                out.push('\n');

                out.push_str("Relation:\n");
                for r in &impact.reasons {
                    out.push_str(&format!(
                        "  {} → {}\n",
                        r.relation.source.0, r.relation.target.0
                    ));
                }
                out.push('\n');

                let conf_str = match impact.confidence {
                    Confidence::High => "high",
                    Confidence::Medium => "medium",
                    Confidence::Low => "low",
                };
                out.push_str(&format!("Confidence:\n  {}\n\n", conf_str));
            }

            if !warnings.is_empty() {
                out.push_str("Warnings:\n");
                for w in warnings {
                    out.push_str(&format!("  {}\n", w));
                }
            }

            out.push_str(
                "\nRun 'agentic-doc scan' once the documentation is updated to record the new baseline.",
            );

            out.trim_end().to_string()
        }
        DocsResult::Coverage { coverage, all, .. } => format_coverage_text(coverage, *all),
    }
}

pub fn format_docs_json(result: &DocsResult) -> String {
    match result {
        DocsResult::Impacts {
            impacts,
            changeset,
            warnings,
            ..
        } => {
            let mut json_impacts = Vec::new();

            for impact in impacts {
                for reason in &impact.reasons {
                    let conf_str = match impact.confidence {
                        Confidence::High => "high",
                        Confidence::Medium => "medium",
                        Confidence::Low => "low",
                    };
                    json_impacts.push(json!({
                        "document": impact.document.0,
                        "reason": reason.change.reason,
                        "relation": {
                            "source": reason.relation.source.0,
                            "target": reason.relation.target.0,
                            "origin": "explicit",
                            "status": "validated"
                        },
                        "confidence": conf_str
                    }));
                }
            }

            let json_val = json!({
                "command": "docs",
                "schema_version": SCHEMA_VERSION,
                "changes": changeset.changes.len(),
                "impacts": json_impacts,
                "warnings": warnings
            });

            serde_json::to_string_pretty(&json_val).unwrap()
        }
        DocsResult::Coverage { coverage, all, .. } => format_coverage_json(coverage, *all),
    }
}

pub fn format_coverage_text(coverage: &crate::domain::coverage::Coverage, all: bool) -> String {
    let mut out = String::from("Documentation coverage\n\nSummary:\n");
    out.push_str(&format!("  {} documented directly\n", coverage.direct));
    out.push_str(&format!("  {} documented via file\n", coverage.via_file));
    out.push_str(&format!("  {} undocumented\n", coverage.undocumented));
    out.push_str(&format!("  {} elements total\n", coverage.total));
    out.push_str(&format!(
        "  coverage: {}%\n\n",
        (coverage.rate * 100.0).round() as u64
    ));

    if all {
        if !coverage.direct_entries.is_empty() {
            out.push_str("Documented directly:\n");
            append_grouped_entries(&mut out, &coverage.direct_entries);
            out.push('\n');
        }
        if !coverage.via_file_entries.is_empty() {
            out.push_str("Documented via file:\n");
            append_grouped_entries(&mut out, &coverage.via_file_entries);
            out.push('\n');
        }
    }

    if !coverage.undocumented_entries.is_empty() {
        out.push_str("Undocumented:\n");
        append_grouped_entries(&mut out, &coverage.undocumented_entries);
        out.push('\n');
    }

    out.push_str(&format!(
        "Unreferenced documents: {}\n",
        coverage.unreferenced_documents.len()
    ));
    for doc in &coverage.unreferenced_documents {
        out.push_str(&format!("  {}\n", doc));
    }

    out.trim_end().to_string()
}

fn append_grouped_entries(out: &mut String, entries: &[CoverageEntry]) {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<&str, Vec<&CoverageEntry>> = BTreeMap::new();
    for entry in entries {
        map.entry(&entry.file).or_default().push(entry);
    }

    for (file, file_entries) in map {
        out.push_str(&format!("  {}  ({})\n", file, file_entries.len()));
        for entry in file_entries {
            out.push_str(&format!("    {}\n", entry.element.0));
        }
        out.push('\n');
    }
}

pub fn format_coverage_json(coverage: &crate::domain::coverage::Coverage, all: bool) -> String {
    let map_entries = |entries: &[CoverageEntry]| {
        entries
            .iter()
            .map(|e| {
                let kind_str = match e.kind {
                    SourceElementKind::File => "file",
                    SourceElementKind::Class => "class",
                    SourceElementKind::Function => "function",
                    SourceElementKind::Method => "method",
                };
                json!({
                    "file": e.file,
                    "element": e.element.0,
                    "kind": kind_str
                })
            })
            .collect::<Vec<_>>()
    };

    let mut json_obj = json!({
        "command": "docs",
        "schema_version": SCHEMA_VERSION,
        "coverage": {
            "total": coverage.total,
            "direct": coverage.direct,
            "via_file": coverage.via_file,
            "undocumented": coverage.undocumented,
            "rate": coverage.rate
        },
        "undocumented": map_entries(&coverage.undocumented_entries),
        "unreferenced_documents": coverage.unreferenced_documents
    });

    if all {
        json_obj.as_object_mut().unwrap().insert(
            "documented_direct".to_string(),
            json!(map_entries(&coverage.direct_entries)),
        );
        json_obj.as_object_mut().unwrap().insert(
            "documented_via_file".to_string(),
            json!(map_entries(&coverage.via_file_entries)),
        );
    }

    serde_json::to_string_pretty(&json_obj).unwrap()
}

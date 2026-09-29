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

/// Version du contrat JSON (cli.md §12, décision 15).
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

use crate::application::relations::{
    AddRelationResult, CheckRelationsResult, ListRelationsResult, RelationResolution,
    RemoveRelationResult,
};

pub fn format_relations_list_text(result: &ListRelationsResult) -> String {
    if result.relations.is_empty() {
        return "No relations declared.".to_string();
    }

    let mut out = String::from("Relations\n\n");
    for info in &result.relations {
        let rel = &info.relation;
        let conf_str = match rel.confidence {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        };

        out.push_str(&format!("  {} → {}", rel.source.0, rel.target.0));

        let res_suffix = match info.resolution {
            RelationResolution::Ok => format!("  ({})", conf_str),
            RelationResolution::SourceNotFound => "  [source not found]".to_string(),
            RelationResolution::TargetNotFound => "  [target not found]".to_string(),
            RelationResolution::SourceAndTargetNotFound => {
                "  [source not found, target not found]".to_string()
            }
        };

        out.push_str(&res_suffix);

        if rel.origin != crate::domain::relation::RelationOrigin::Explicit
            || rel.status != crate::domain::relation::RelationStatus::Validated
        {
            let origin_str = match rel.origin {
                crate::domain::relation::RelationOrigin::Explicit => "explicit",
                crate::domain::relation::RelationOrigin::Discovered => "discovered",
                crate::domain::relation::RelationOrigin::Imported => "imported",
            };
            let status_str = match rel.status {
                crate::domain::relation::RelationStatus::Candidate => "candidate",
                crate::domain::relation::RelationStatus::Validated => "validated",
                crate::domain::relation::RelationStatus::Rejected => "rejected",
            };
            out.push_str(&format!("  [{}/{}]", origin_str, status_str));
        }
        out.push('\n');
    }

    out.trim_end().to_string()
}

pub fn format_relations_list_json(result: &ListRelationsResult) -> String {
    let json_relations: Vec<_> = result
        .relations
        .iter()
        .map(|info| {
            let rel = &info.relation;
            let conf_str = match rel.confidence {
                Confidence::High => "high",
                Confidence::Medium => "medium",
                Confidence::Low => "low",
            };
            let origin_str = match rel.origin {
                crate::domain::relation::RelationOrigin::Explicit => "explicit",
                crate::domain::relation::RelationOrigin::Discovered => "discovered",
                crate::domain::relation::RelationOrigin::Imported => "imported",
            };
            let status_str = match rel.status {
                crate::domain::relation::RelationStatus::Candidate => "candidate",
                crate::domain::relation::RelationStatus::Validated => "validated",
                crate::domain::relation::RelationStatus::Rejected => "rejected",
            };
            let res_str = match info.resolution {
                RelationResolution::Ok => "ok",
                RelationResolution::SourceNotFound => "source_not_found",
                RelationResolution::TargetNotFound => "target_not_found",
                RelationResolution::SourceAndTargetNotFound => "source_and_target_not_found",
            };

            json!({
                "source": rel.source.0,
                "target": rel.target.0,
                "origin": origin_str,
                "status": status_str,
                "confidence": conf_str,
                "resolution": res_str
            })
        })
        .collect();

    let json_val = json!({
        "command": "relations",
        "schema_version": SCHEMA_VERSION,
        "count": result.relations.len(),
        "relations": json_relations
    });

    serde_json::to_string_pretty(&json_val).unwrap()
}

pub fn format_relations_check_text(result: &CheckRelationsResult) -> String {
    let mut out = String::from("Relations check\n\n");
    let summary = &result.summary;
    let total_relations = result.relations.len();

    out.push_str(&format!("  {} relations\n", total_relations));
    out.push_str(&format!(
        "  {} source not found\n",
        summary.sources_not_found
    ));
    out.push_str(&format!(
        "  {} target not found\n",
        summary.targets_not_found
    ));
    out.push_str(&format!("  {} duplicates\n\n", summary.duplicates));

    let mut sources_not_found = Vec::new();
    let mut targets_not_found = Vec::new();

    for info in &result.relations {
        let rel = &info.relation;
        match info.resolution {
            RelationResolution::SourceNotFound => {
                sources_not_found.push(format!("  {} → {}", rel.source.0, rel.target.0));
            }
            RelationResolution::TargetNotFound => {
                targets_not_found.push(format!("  {} → {}", rel.source.0, rel.target.0));
            }
            RelationResolution::SourceAndTargetNotFound => {
                sources_not_found.push(format!("  {} → {}", rel.source.0, rel.target.0));
                targets_not_found.push(format!("  {} → {}", rel.source.0, rel.target.0));
            }
            RelationResolution::Ok => {}
        }
    }

    if !sources_not_found.is_empty() {
        out.push_str("Sources not found:\n");
        for line in sources_not_found {
            out.push_str(&format!("{}\n", line));
        }
        out.push('\n');
    }

    if !targets_not_found.is_empty() {
        out.push_str("Targets not found:\n");
        for line in targets_not_found {
            out.push_str(&format!("{}\n", line));
        }
        out.push('\n');
    }

    out.trim_end().to_string()
}

pub fn format_relations_check_json(result: &CheckRelationsResult) -> String {
    let json_relations: Vec<_> = result
        .relations
        .iter()
        .map(|info| {
            let rel = &info.relation;
            let conf_str = match rel.confidence {
                Confidence::High => "high",
                Confidence::Medium => "medium",
                Confidence::Low => "low",
            };
            let origin_str = match rel.origin {
                crate::domain::relation::RelationOrigin::Explicit => "explicit",
                crate::domain::relation::RelationOrigin::Discovered => "discovered",
                crate::domain::relation::RelationOrigin::Imported => "imported",
            };
            let status_str = match rel.status {
                crate::domain::relation::RelationStatus::Candidate => "candidate",
                crate::domain::relation::RelationStatus::Validated => "validated",
                crate::domain::relation::RelationStatus::Rejected => "rejected",
            };
            let res_str = match info.resolution {
                RelationResolution::Ok => "ok",
                RelationResolution::SourceNotFound => "source_not_found",
                RelationResolution::TargetNotFound => "target_not_found",
                RelationResolution::SourceAndTargetNotFound => "source_and_target_not_found",
            };

            json!({
                "source": rel.source.0,
                "target": rel.target.0,
                "origin": origin_str,
                "status": status_str,
                "confidence": conf_str,
                "resolution": res_str
            })
        })
        .collect();

    let json_val = json!({
        "command": "relations",
        "schema_version": SCHEMA_VERSION,
        "count": result.relations.len(),
        "relations": json_relations,
        "summary": {
            "ok": result.summary.ok,
            "sources_not_found": result.summary.sources_not_found,
            "targets_not_found": result.summary.targets_not_found,
            "duplicates": result.summary.duplicates
        }
    });

    serde_json::to_string_pretty(&json_val).unwrap()
}

pub fn format_relations_add_text(result: &AddRelationResult) -> String {
    let rel = &result.relation;
    let mut out = match result.result {
        crate::domain::relation::InsertOutcome::Added => {
            format!("Relation added: {} → {}", rel.source.0, rel.target.0)
        }
        crate::domain::relation::InsertOutcome::Updated => {
            let conf_str = match rel.confidence {
                Confidence::High => "high",
                Confidence::Medium => "medium",
                Confidence::Low => "low",
            };
            format!(
                "Relation updated: {} → {} (confidence {})",
                rel.source.0, rel.target.0, conf_str
            )
        }
        crate::domain::relation::InsertOutcome::Unchanged => {
            format!(
                "Relation already declared: {} → {}",
                rel.source.0, rel.target.0
            )
        }
    };

    if result.dry_run {
        out.push_str("\n\nDry run: nothing written.");
    }

    out
}

pub fn format_relations_add_json(result: &AddRelationResult) -> String {
    let rel = &result.relation;
    let conf_str = match rel.confidence {
        Confidence::High => "high",
        Confidence::Medium => "medium",
        Confidence::Low => "low",
    };

    let res_str = match result.result {
        crate::domain::relation::InsertOutcome::Added => "added",
        crate::domain::relation::InsertOutcome::Updated => "updated",
        crate::domain::relation::InsertOutcome::Unchanged => "unchanged",
    };

    let json_val = json!({
        "command": "relations",
        "schema_version": SCHEMA_VERSION,
        "dry_run": result.dry_run,
        "result": res_str,
        "relation": {
            "source": rel.source.0,
            "target": rel.target.0,
            "confidence": conf_str
        }
    });

    serde_json::to_string_pretty(&json_val).unwrap()
}

pub fn format_relations_remove_text(result: &RemoveRelationResult) -> String {
    let mut out = if result.removed {
        format!(
            "Relation removed: {} → {}",
            result.source.0, result.target.0
        )
    } else {
        "Nothing to remove.".to_string()
    };

    if result.dry_run {
        out.push_str("\n\nDry run: nothing written.");
    }

    out
}

pub fn format_relations_remove_json(result: &RemoveRelationResult) -> String {
    let json_val = json!({
        "command": "relations",
        "schema_version": SCHEMA_VERSION,
        "dry_run": result.dry_run,
        "removed": result.removed,
        "relation": {
            "source": result.source.0,
            "target": result.target.0
        }
    });

    serde_json::to_string_pretty(&json_val).unwrap()
}

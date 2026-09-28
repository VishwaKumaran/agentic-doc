//! Détection des changements entre un snapshot et un modèle source.

use crate::domain::snapshot::Snapshot;
use crate::domain::source::{ElementId, SourceElementKind, SourceModel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub kind: ChangeKind,
    pub element_id: ElementId,
    pub element_kind: SourceElementKind,
    pub file: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet {
    pub changes: Vec<Change>,
    pub unchanged_files: Vec<String>,
}

pub struct ChangeDetector;

impl ChangeDetector {
    /// Détecte les changements entre un état précédent (optionnel) et l'état courant.
    ///
    /// Si `previous` est `None` (premier run, aucun snapshot), l'état précédent est
    /// considéré comme vide : tous les éléments courants sont rapportés comme `Added`.
    pub fn detect(previous: Option<&Snapshot>, current: &SourceModel) -> ChangeSet {
        let mut changes = Vec::new();
        let mut affected_files = std::collections::HashSet::new();

        // 1. Construction de la map des éléments précédents (vide si premier run)
        let prev_map: std::collections::BTreeMap<_, _> = match previous {
            Some(snap) => snap
                .model
                .files
                .iter()
                .chain(snap.model.elements.iter())
                .map(|e| (&e.id, e))
                .collect(),
            None => std::collections::BTreeMap::new(),
        };

        let curr_elements = current.files.iter().chain(current.elements.iter());
        let curr_map: std::collections::BTreeMap<_, _> =
            curr_elements.map(|e| (&e.id, e)).collect();

        // Ajouts et modifications
        for (id, curr_elem) in &curr_map {
            if let Some(prev_elem) = prev_map.get(id) {
                if prev_elem.content_hash != curr_elem.content_hash {
                    let reason = if curr_elem.kind == SourceElementKind::File {
                        format!("{} was modified.", curr_elem.file)
                    } else {
                        format!("{} was modified.", curr_elem.name)
                    };
                    changes.push(Change {
                        kind: ChangeKind::Modified,
                        element_id: (*id).clone(),
                        element_kind: curr_elem.kind,
                        file: curr_elem.file.clone(),
                        reason,
                    });
                    affected_files.insert(curr_elem.file.clone());
                }
            } else {
                let reason = if curr_elem.kind == SourceElementKind::File {
                    format!("{} was added.", curr_elem.file)
                } else {
                    format!("{} was added.", curr_elem.name)
                };
                changes.push(Change {
                    kind: ChangeKind::Added,
                    element_id: (*id).clone(),
                    element_kind: curr_elem.kind,
                    file: curr_elem.file.clone(),
                    reason,
                });
                affected_files.insert(curr_elem.file.clone());
            }
        }

        // Suppressions
        for (id, prev_elem) in &prev_map {
            if !curr_map.contains_key(id) {
                let reason = if prev_elem.kind == SourceElementKind::File {
                    format!("{} was removed.", prev_elem.file)
                } else {
                    format!("{} was removed.", prev_elem.name)
                };
                changes.push(Change {
                    kind: ChangeKind::Removed,
                    element_id: (*id).clone(),
                    element_kind: prev_elem.kind,
                    file: prev_elem.file.clone(),
                    reason,
                });
                affected_files.insert(prev_elem.file.clone());
            }
        }

        // Tri déterministe par (file, element_id)
        changes.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then_with(|| a.element_id.cmp(&b.element_id))
        });

        // 2. Fichiers inchangés
        let mut unchanged_files: Vec<String> = current
            .files
            .iter()
            .map(|f| f.file.clone())
            .filter(|f| !affected_files.contains(f))
            .collect();
        unchanged_files.sort();
        unchanged_files.dedup();

        ChangeSet {
            changes,
            unchanged_files,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::project::Project;
    use crate::domain::source::{SourceElement, SourceElementKind};

    #[test]
    fn test_identical_snapshots_produce_empty_changeset() {
        let current_dir = std::env::current_dir().unwrap();
        let project = Project::new(&current_dir).unwrap();
        let file_elem = SourceElement {
            id: ElementId::for_file("src/main.rs"),
            kind: SourceElementKind::File,
            name: "src/main.rs".to_string(),
            file: "src/main.rs".to_string(),
            line_start: 1,
            line_end: 10,
            content_hash: 100,
        };
        let model = SourceModel::new(project.id.clone(), vec![file_elem], vec![]);
        let snapshot = Snapshot::new(&project, model.clone(), "2026-09-25T20:00:00.000Z");

        let changeset = ChangeDetector::detect(Some(&snapshot), &model);
        assert!(changeset.changes.is_empty());
        assert_eq!(changeset.unchanged_files, vec!["src/main.rs"]);
    }

    #[test]
    fn test_added_function_and_modified_file() {
        let current_dir = std::env::current_dir().unwrap();
        let project = Project::new(&current_dir).unwrap();

        let prev_file = SourceElement {
            id: ElementId::for_file("src/lib.rs"),
            kind: SourceElementKind::File,
            name: "src/lib.rs".to_string(),
            file: "src/lib.rs".to_string(),
            line_start: 1,
            line_end: 5,
            content_hash: 100,
        };
        let prev_model = SourceModel::new(project.id.clone(), vec![prev_file], vec![]);
        let snapshot = Snapshot::new(&project, prev_model, "2026-09-25T20:00:00.000Z");

        let curr_file = SourceElement {
            id: ElementId::for_file("src/lib.rs"),
            kind: SourceElementKind::File,
            name: "src/lib.rs".to_string(),
            file: "src/lib.rs".to_string(),
            line_start: 1,
            line_end: 10,
            content_hash: 200, // file content changed
        };
        let curr_func = SourceElement {
            id: ElementId::for_symbol("src/lib.rs", "new_fn"),
            kind: SourceElementKind::Function,
            name: "new_fn".to_string(),
            file: "src/lib.rs".to_string(),
            line_start: 6,
            line_end: 10,
            content_hash: 300,
        };
        let curr_model = SourceModel::new(project.id.clone(), vec![curr_file], vec![curr_func]);

        let changeset = ChangeDetector::detect(Some(&snapshot), &curr_model);
        assert_eq!(changeset.changes.len(), 2);
        assert_eq!(changeset.changes[0].kind, ChangeKind::Modified);
        assert_eq!(
            changeset.changes[0].element_id,
            ElementId::for_file("src/lib.rs")
        );
        assert_eq!(changeset.changes[1].kind, ChangeKind::Added);
        assert_eq!(
            changeset.changes[1].element_id,
            ElementId::for_symbol("src/lib.rs", "new_fn")
        );
        assert!(changeset.unchanged_files.is_empty());
    }

    #[test]
    fn test_premier_run_sans_snapshot_tous_elements_added() {
        // Premier run : aucun snapshot précédent → tous les éléments sont Added
        let current_dir = std::env::current_dir().unwrap();
        let project = Project::new(&current_dir).unwrap();

        let file_elem = SourceElement {
            id: ElementId::for_file("src/app.py"),
            kind: SourceElementKind::File,
            name: "src/app.py".to_string(),
            file: "src/app.py".to_string(),
            line_start: 1,
            line_end: 5,
            content_hash: 42,
        };
        let func_elem = SourceElement {
            id: ElementId::for_symbol("src/app.py", "main"),
            kind: SourceElementKind::Function,
            name: "main".to_string(),
            file: "src/app.py".to_string(),
            line_start: 1,
            line_end: 5,
            content_hash: 43,
        };
        let model = SourceModel::new(project.id.clone(), vec![file_elem], vec![func_elem]);

        let changeset = ChangeDetector::detect(None, &model);
        // Tous les éléments doivent être Added
        assert_eq!(changeset.changes.len(), 2);
        assert!(changeset
            .changes
            .iter()
            .all(|c| c.kind == ChangeKind::Added));
        // Aucun fichier inchangé au premier run
        assert!(changeset.unchanged_files.is_empty());
    }
}

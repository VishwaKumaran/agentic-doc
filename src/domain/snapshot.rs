//! Définition d'un snapshot de projet et de son identifiant.

use crate::domain::project::Project;
use crate::domain::source::SourceModel;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SnapshotId(pub String);

impl SnapshotId {
    pub fn new(iso_timestamp: &str) -> Self {
        // Remplace les ':' par '-' pour garantir un tri lexicographique équivalent au tri chronologique
        let formatted = iso_timestamp.replace(':', "-");
        Self(formatted)
    }
}

impl fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub project_id: String,
    pub created_at: String,
    pub model: SourceModel,
}

impl Snapshot {
    pub fn new(project: &Project, model: SourceModel, created_at: &str) -> Self {
        let id = SnapshotId::new(created_at);
        Self {
            id,
            project_id: project.id.clone(),
            created_at: created_at.to_string(),
            model,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::source::{ElementId, SourceElement, SourceElementKind};

    #[test]
    fn test_snapshot_id_lexicographical_order() {
        let t1 = SnapshotId::new("2026-09-25T20:00:00.000Z");
        let t2 = SnapshotId::new("2026-09-25T21:00:00.000Z");
        assert_eq!(t1.0, "2026-09-25T20-00-00.000Z");
        assert!(t1 < t2);
    }

    #[test]
    fn test_snapshot_serialization() {
        let current = std::env::current_dir().unwrap();
        let project = Project::new(&current).unwrap();
        let file_elem = SourceElement {
            id: ElementId::for_file("src/lib.rs"),
            kind: SourceElementKind::File,
            name: "src/lib.rs".to_string(),
            file: "src/lib.rs".to_string(),
            line_start: 1,
            line_end: 10,
            content_hash: 999,
        };
        let model = SourceModel::new(project.id.clone(), vec![file_elem], vec![]);
        let snapshot = Snapshot::new(&project, model, "2026-09-25T20:58:00.000Z");

        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: Snapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot, deserialized);
    }
}

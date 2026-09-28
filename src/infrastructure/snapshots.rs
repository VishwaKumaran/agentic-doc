//! Persistance des snapshots dans le système de fichiers (.agentic-doc/snapshots/).

use crate::domain::project::Project;
use crate::domain::snapshot::{Snapshot, SnapshotId};
use crate::infrastructure::filesystem::StorageError;
use std::fs;
use std::path::PathBuf;

pub trait SnapshotRepository {
    fn save(&self, snapshot: &Snapshot) -> Result<(), StorageError>;
    fn get_latest(&self, project: &Project) -> Result<Option<Snapshot>, StorageError>;
    fn list(&self, project: &Project) -> Result<Vec<SnapshotId>, StorageError>;
}

pub struct FileSystemSnapshotRepository {
    snapshots_dir: PathBuf,
}

impl FileSystemSnapshotRepository {
    pub fn new(workspace_root: PathBuf) -> Self {
        let snapshots_dir = workspace_root.join(".agentic-doc").join("snapshots");
        Self { snapshots_dir }
    }
}

impl SnapshotRepository for FileSystemSnapshotRepository {
    fn save(&self, snapshot: &Snapshot) -> Result<(), StorageError> {
        if !self.snapshots_dir.exists() {
            fs::create_dir_all(&self.snapshots_dir)?;
        }

        let target_path = self.snapshots_dir.join(format!("{}.json", snapshot.id.0));
        let temp_path = self.snapshots_dir.join(format!("{}.tmp", snapshot.id.0));

        let content = serde_json::to_string_pretty(snapshot)?;
        fs::write(&temp_path, content)?;
        fs::rename(&temp_path, &target_path)?;

        Ok(())
    }

    fn get_latest(&self, project: &Project) -> Result<Option<Snapshot>, StorageError> {
        if !self.snapshots_dir.exists() {
            return Ok(None);
        }

        let mut snapshots = Vec::new();
        for entry in fs::read_dir(&self.snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                let snapshot: Snapshot = serde_json::from_str(&content).map_err(|e| {
                    StorageError::CorruptedData(format!("{}: {}", path.display(), e))
                })?;
                if snapshot.project_id == project.id {
                    snapshots.push(snapshot);
                }
            }
        }

        snapshots.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(snapshots.pop())
    }

    fn list(&self, project: &Project) -> Result<Vec<SnapshotId>, StorageError> {
        if !self.snapshots_dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                let snapshot: Snapshot = serde_json::from_str(&content).map_err(|e| {
                    StorageError::CorruptedData(format!("{}: {}", path.display(), e))
                })?;
                if snapshot.project_id == project.id {
                    ids.push(snapshot.id);
                }
            }
        }

        ids.sort();
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::source::{ElementId, SourceElement, SourceElementKind, SourceModel};
    use tempfile::tempdir;

    #[test]
    fn test_save_and_get_latest_snapshot() {
        let temp = tempdir().unwrap();
        let project = Project::new(&temp).unwrap();
        let repo = FileSystemSnapshotRepository::new(temp.path().to_path_buf());

        let file_elem = SourceElement {
            id: ElementId::for_file("src/main.rs"),
            kind: SourceElementKind::File,
            name: "src/main.rs".to_string(),
            file: "src/main.rs".to_string(),
            line_start: 1,
            line_end: 10,
            content_hash: 1234,
        };
        let model = SourceModel::new(project.id.clone(), vec![file_elem], vec![]);

        let s1 = Snapshot::new(&project, model.clone(), "2026-09-25T10:00:00.000Z");
        let s2 = Snapshot::new(&project, model, "2026-09-25T11:00:00.000Z");

        repo.save(&s1).unwrap();
        repo.save(&s2).unwrap();

        let latest = repo.get_latest(&project).unwrap().unwrap();
        assert_eq!(latest.id, s2.id);

        let list = repo.list(&project).unwrap();
        assert_eq!(list, vec![s1.id, s2.id]);
    }
}

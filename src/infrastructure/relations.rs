//! Chargement et sauvegarde des relations explicites dans .agentic-doc/relations.json.

use crate::domain::relation::{DocumentationRelation, RelationSet};
use crate::infrastructure::filesystem::StorageError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct RelationsFileDto {
    pub version: u32,
    pub relations: Vec<DocumentationRelation>,
}

pub trait RelationRepository {
    fn load(&self) -> Result<RelationSet, StorageError>;
    fn save(&self, relations: &RelationSet) -> Result<(), StorageError>;
}

pub struct FileSystemRelationRepository {
    relations_file: PathBuf,
}

impl FileSystemRelationRepository {
    pub fn new(workspace_root: PathBuf) -> Self {
        let relations_file = workspace_root.join(".agentic-doc").join("relations.json");
        Self { relations_file }
    }
}

impl RelationRepository for FileSystemRelationRepository {
    fn load(&self) -> Result<RelationSet, StorageError> {
        if !self.relations_file.exists() {
            return Ok(RelationSet::default());
        }

        let content = fs::read_to_string(&self.relations_file)?;
        let dto: RelationsFileDto = serde_json::from_str(&content).map_err(|e| {
            StorageError::InvalidRelations(format!("{}: {}", self.relations_file.display(), e))
        })?;

        Ok(RelationSet::new(dto.relations))
    }

    fn save(&self, relations: &RelationSet) -> Result<(), StorageError> {
        if let Some(parent) = self.relations_file.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let dto = RelationsFileDto {
            version: 1,
            relations: relations.relations.clone(),
        };

        let content = serde_json::to_string_pretty(&dto)?;
        fs::write(&self.relations_file, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::document::DocumentId;
    use crate::domain::relation::DocumentationRelation;
    use crate::domain::source::ElementId;
    use tempfile::tempdir;

    #[test]
    fn test_missing_relations_file_returns_empty_set() {
        let temp = tempdir().unwrap();
        let repo = FileSystemRelationRepository::new(temp.path().to_path_buf());

        let set = repo.load().unwrap();
        assert!(set.relations.is_empty());
    }

    #[test]
    fn test_invalid_relations_file_returns_error() {
        let temp = tempdir().unwrap();
        let dir = temp.path().join(".agentic-doc");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("relations.json"), "{ invalid json }").unwrap();

        let repo = FileSystemRelationRepository::new(temp.path().to_path_buf());
        let err = repo.load().unwrap_err();

        assert!(matches!(err, StorageError::InvalidRelations(_)));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp = tempdir().unwrap();
        let repo = FileSystemRelationRepository::new(temp.path().to_path_buf());

        let rel = DocumentationRelation::explicit(
            ElementId::for_symbol("src/auth.py", "login"),
            DocumentId("auth.md".to_string()),
        );
        let set = RelationSet::new(vec![rel]);

        repo.save(&set).unwrap();
        let loaded = repo.load().unwrap();

        assert_eq!(set, loaded);
    }
}

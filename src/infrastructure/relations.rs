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
        // 1. Refuser d'écraser un fichier existant illisible (cli.md §2)
        if self.relations_file.exists() {
            self.load()?;
        }

        let parent = self.relations_file.parent().ok_or_else(|| {
            StorageError::InvalidRelations("Chemin de fichier de relations invalide".to_string())
        })?;

        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }

        // 2. Dédoublonnage et tri canonique
        let mut clean_set = RelationSet::new(Vec::new());
        for rel in &relations.relations {
            clean_set.insert(rel.clone());
        }

        let dto = RelationsFileDto {
            version: 1,
            relations: clean_set.relations,
        };

        let content = serde_json::to_string_pretty(&dto)?;

        // 3. Écriture atomique (fichier temporaire puis renommage)
        let temp_file_name = format!(
            ".relations.tmp.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let temp_file_path = parent.join(temp_file_name);

        fs::write(&temp_file_path, content)?;
        fs::rename(&temp_file_path, &self.relations_file)?;

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
    fn test_save_refuses_to_overwrite_invalid_file_and_preserves_content() {
        let temp = tempdir().unwrap();
        let dir = temp.path().join(".agentic-doc");
        fs::create_dir_all(&dir).unwrap();
        let rel_file = dir.join("relations.json");
        let invalid_content = "{ invalid json }";
        fs::write(&rel_file, invalid_content).unwrap();

        let repo = FileSystemRelationRepository::new(temp.path().to_path_buf());
        let set = RelationSet::new(vec![DocumentationRelation::explicit(
            ElementId::for_symbol("src/auth.py", "login"),
            DocumentId("auth.md".to_string()),
        )]);

        let err = repo.save(&set).unwrap_err();
        assert!(matches!(err, StorageError::InvalidRelations(_)));
        assert_eq!(fs::read_to_string(&rel_file).unwrap(), invalid_content);
    }

    #[test]
    fn test_save_creates_file_and_directory_when_missing() {
        let temp = tempdir().unwrap();
        let repo = FileSystemRelationRepository::new(temp.path().to_path_buf());

        let rel = DocumentationRelation::explicit(
            ElementId::for_symbol("src/auth.py", "login"),
            DocumentId("auth.md".to_string()),
        );
        let set = RelationSet::new(vec![rel]);

        repo.save(&set).unwrap();
        assert!(temp.path().join(".agentic-doc/relations.json").exists());
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

    #[test]
    fn test_save_deduplicates_and_sorts_canonically() {
        let temp = tempdir().unwrap();
        let repo = FileSystemRelationRepository::new(temp.path().to_path_buf());

        let r1 = DocumentationRelation::explicit(
            ElementId::for_symbol("src/users.py", "User"),
            DocumentId("users.md".to_string()),
        );
        let r2 = DocumentationRelation::explicit(
            ElementId::for_symbol("src/auth.py", "login"),
            DocumentId("auth.md".to_string()),
        );
        let r3 = r2.clone();

        let set = RelationSet {
            relations: vec![r1, r2, r3],
        };

        repo.save(&set).unwrap();
        let loaded = repo.load().unwrap();

        assert_eq!(loaded.relations.len(), 2);
        assert_eq!(loaded.relations[0].source.0, "src/auth.py::login");
        assert_eq!(loaded.relations[1].source.0, "src/users.py::User");
    }

    #[test]
    fn test_save_is_idempotent() {
        let temp = tempdir().unwrap();
        let repo = FileSystemRelationRepository::new(temp.path().to_path_buf());

        let rel = DocumentationRelation::explicit(
            ElementId::for_symbol("src/auth.py", "login"),
            DocumentId("auth.md".to_string()),
        );
        let set = RelationSet::new(vec![rel]);

        repo.save(&set).unwrap();
        let content1 = fs::read_to_string(temp.path().join(".agentic-doc/relations.json")).unwrap();

        repo.save(&set).unwrap();
        let content2 = fs::read_to_string(temp.path().join(".agentic-doc/relations.json")).unwrap();

        assert_eq!(content1, content2);
    }
}

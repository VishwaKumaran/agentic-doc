//! Entité représentant le projet analysé.

use crate::domain::error::DomainError;
use crate::domain::hash::fnv1a_64;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Project {
    pub id: String,
    pub root: PathBuf,
    pub name: String,
}

impl Project {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, DomainError> {
        let root_path = root.as_ref();
        let canonical = root_path
            .canonicalize()
            .map_err(|e| DomainError::InvalidProject(format!("{}: {}", root_path.display(), e)))?;

        let name = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();

        let hash = fnv1a_64(canonical.to_string_lossy().as_bytes());
        let id = format!("{}-{:016x}", name, hash);

        Ok(Self {
            id,
            root: canonical,
            name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_creation() {
        let current = std::env::current_dir().unwrap();
        let project = Project::new(&current).unwrap();
        assert!(!project.id.is_empty());
        assert_eq!(project.root, current.canonicalize().unwrap());
    }
}

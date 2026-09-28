//! Configuration du workspace dans .agentic-doc/config.json.

use crate::infrastructure::filesystem::StorageError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub version: u32,
    pub project_root: String,
    pub docs_root: String,
}

impl WorkspaceConfig {
    pub fn new(project_root: String, docs_root: String) -> Self {
        Self {
            version: 1,
            project_root,
            docs_root,
        }
    }

    pub fn load(workspace: &Path) -> Result<Self, StorageError> {
        let config_path = workspace.join(".agentic-doc").join("config.json");
        if !config_path.exists() {
            return Err(StorageError::ConfigNotFound);
        }
        let content = fs::read_to_string(config_path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, workspace: &Path) -> Result<(), StorageError> {
        let dir = workspace.join(".agentic-doc");
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let config_path = dir.join("config.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }
}

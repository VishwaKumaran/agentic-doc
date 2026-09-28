//! Port (trait) pour l'analyse des répertoires de documentation.

use crate::domain::document::Document;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocumentationError {
    #[error("Erreur E/S lors de l'analyse de la documentation : {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur lors de la navigation dans les fichiers de documentation : {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("Racine de la documentation introuvable : {0}")]
    RootNotFound(String),
}

pub trait DocumentationAnalyzer {
    fn analyze(&self, docs_root: &Path) -> Result<Vec<Document>, DocumentationError>;
}

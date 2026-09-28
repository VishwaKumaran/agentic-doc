//! Port (trait) pour l'analyse de projets de code source.

use crate::domain::project::Project;
use crate::domain::source::SourceModel;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("Erreur E/S lors de l'analyse : {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur lors de la navigation dans le répertoire : {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("Racine du projet introuvable : {0}")]
    RootNotFound(String),

    #[error("Fichier ilisible : {0}")]
    UnreadableFile(String),
}

pub trait ProjectAnalyzer {
    fn analyze(&self, project: &Project) -> Result<SourceModel, AnalysisError>;
}

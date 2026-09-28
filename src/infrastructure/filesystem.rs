//! Modèle d'erreurs d'infrastructure / persistance.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Erreur d'E/S de stockage : {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur de sérialisation JSON : {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Structure de données corrompue dans le stockage : {0}")]
    CorruptedData(String),

    #[error("Relations invalides dans relations.json : {0}")]
    InvalidRelations(String),

    #[error("Configuration introuvable")]
    ConfigNotFound,
}

//! Modèle d'erreurs du domaine.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum DomainError {
    #[error("Identifiant d'élément invalide : {0}")]
    InvalidElementId(String),

    #[error("Élément en double : {0}")]
    DuplicateElementId(String),

    #[error("Projet invalide : {0}")]
    InvalidProject(String),
}

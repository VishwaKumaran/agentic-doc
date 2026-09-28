//! Structures de données du domaine pour l'analyse d'impact.

use crate::domain::change::Change;
use crate::domain::document::DocumentId;
use crate::domain::relation::{Confidence, DocumentationRelation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactReason {
    pub change: Change,
    pub relation: DocumentationRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Impact {
    pub document: DocumentId,
    pub reasons: Vec<ImpactReason>,
    pub confidence: Confidence,
}

//! Relations entre éléments de code source et documents.

use crate::domain::document::{Document, DocumentId};
use crate::domain::source::{ElementId, SourceModel};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationOrigin {
    Explicit,
    Discovered,
    Imported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationStatus {
    Candidate,
    Validated,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn downgrade(self) -> Self {
        match self {
            Confidence::High => Confidence::Medium,
            Confidence::Medium => Confidence::Low,
            Confidence::Low => Confidence::Low,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentationRelation {
    pub source: ElementId,
    pub target: DocumentId,
    pub origin: RelationOrigin,
    pub status: RelationStatus,
    pub confidence: Confidence,
}

impl DocumentationRelation {
    pub fn explicit(source: ElementId, target: DocumentId) -> Self {
        Self {
            source,
            target,
            origin: RelationOrigin::Explicit,
            status: RelationStatus::Validated,
            confidence: Confidence::High,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationSet {
    pub relations: Vec<DocumentationRelation>,
}

impl RelationSet {
    pub fn new(mut relations: Vec<DocumentationRelation>) -> Self {
        relations.sort_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then_with(|| a.target.cmp(&b.target))
        });
        Self { relations }
    }

    pub fn relations_for_source(&self, source: &ElementId) -> Vec<&DocumentationRelation> {
        self.relations
            .iter()
            .filter(|r| r.source == *source)
            .collect()
    }

    pub fn relations_for_target(&self, target: &DocumentId) -> Vec<&DocumentationRelation> {
        self.relations
            .iter()
            .filter(|r| r.target == *target)
            .collect()
    }

    /// Relations qui ne se résolvent pas contre l'état courant (cli.md §7).
    ///
    /// Une relation est signalée si sa `source` ne correspond à aucun élément connu
    /// (`File`, classe, fonction ou méthode), ou si sa `target` ne correspond à aucun
    /// document connu.
    ///
    /// Ces relations sont ignorées par l'analyse d'impact **et** par le calcul de
    /// couverture : l'avertissement est donc leur seul retour visible, et la seule façon
    /// de détecter une faute de frappe dans un identifiant.
    ///
    /// Le résultat est trié et dédoublonné, pour que deux exécutions produisent la même
    /// sortie.
    pub fn unresolved(&self, model: &SourceModel, documents: &[Document]) -> Vec<String> {
        let known_elements: BTreeSet<&str> = model
            .files
            .iter()
            .chain(model.elements.iter())
            .map(|e| e.id.0.as_str())
            .collect();
        let known_documents: BTreeSet<&str> = documents.iter().map(|d| d.id.0.as_str()).collect();

        let mut warnings = Vec::new();
        for rel in &self.relations {
            if !known_elements.contains(rel.source.0.as_str()) {
                warnings.push(format!("relation source not found: {}", rel.source.0));
            }
            if !known_documents.contains(rel.target.0.as_str()) {
                warnings.push(format!("relation target not found: {}", rel.target.0));
            }
        }

        warnings.sort();
        warnings.dedup();
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_set_multiple_sources_and_targets() {
        let e1 = ElementId::for_symbol("src/auth.py", "login");
        let e2 = ElementId::for_symbol("src/auth.py", "logout");
        let d1 = DocumentId("auth.md".to_string());
        let d2 = DocumentId("security.md".to_string());

        let r1 = DocumentationRelation::explicit(e1.clone(), d1.clone());
        let r2 = DocumentationRelation::explicit(e1.clone(), d2.clone());
        let r3 = DocumentationRelation::explicit(e2.clone(), d1.clone());

        let set = RelationSet::new(vec![r1.clone(), r2.clone(), r3.clone()]);

        assert_eq!(set.relations_for_source(&e1).len(), 2);
        assert_eq!(set.relations_for_target(&d1).len(), 2);
    }

    #[test]
    fn test_unresolved_detects_orphan_source_and_orphan_target() {
        use crate::domain::source::{SourceElement, SourceElementKind};
        use std::path::PathBuf;

        let model = SourceModel::new(
            "proj".to_string(),
            vec![SourceElement {
                id: ElementId::for_file("src/auth.py"),
                kind: SourceElementKind::File,
                name: "src/auth.py".to_string(),
                file: "src/auth.py".to_string(),
                line_start: 1,
                line_end: 10,
                content_hash: 1,
            }],
            vec![SourceElement {
                id: ElementId::for_symbol("src/auth.py", "login"),
                kind: SourceElementKind::Function,
                name: "login".to_string(),
                file: "src/auth.py".to_string(),
                line_start: 1,
                line_end: 5,
                content_hash: 2,
            }],
        );

        let docs = vec![Document::new(
            "auth.md",
            Some("Auth".to_string()),
            PathBuf::from("auth.md"),
        )];

        let valid_rel = DocumentationRelation::explicit(
            ElementId::for_symbol("src/auth.py", "login"),
            DocumentId("auth.md".to_string()),
        );
        let orphan_src = DocumentationRelation::explicit(
            ElementId::for_symbol("src/legacy.py", "OldAuth"),
            DocumentId("auth.md".to_string()),
        );
        let orphan_target = DocumentationRelation::explicit(
            ElementId::for_symbol("src/auth.py", "login"),
            DocumentId("removed-doc.md".to_string()),
        );

        let set = RelationSet::new(vec![valid_rel, orphan_src, orphan_target]);
        let warnings = set.unresolved(&model, &docs);

        assert_eq!(
            warnings,
            vec![
                "relation source not found: src/legacy.py::OldAuth".to_string(),
                "relation target not found: removed-doc.md".to_string(),
            ]
        );
    }
}

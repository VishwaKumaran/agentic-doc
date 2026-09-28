//! Calcul et analyse d'impact entre changements et documentation.

use crate::domain::change::ChangeSet;
use crate::domain::document::Document;
use crate::domain::impact::{Impact, ImpactReason};
use crate::domain::relation::{Confidence, RelationSet};
use crate::domain::source::{ElementId, SourceElementKind};
use std::collections::{BTreeMap, HashSet};

pub struct ImpactAnalyzer;

impl ImpactAnalyzer {
    /// Analyse les impacts des changements sur la documentation.
    ///
    /// Ne produit **que** des impacts. Le contrôle de résolution des relations vit dans
    /// `RelationSet::unresolved` : un seul message pour un seul endroit, disponible en
    /// mode impact comme en mode couverture (cli.md §7).
    pub fn analyze(
        changes: &ChangeSet,
        relations: &RelationSet,
        documents: &[Document],
    ) -> Vec<Impact> {
        let doc_map: HashSet<_> = documents.iter().map(|d| &d.id).collect();
        let mut doc_impacts: BTreeMap<_, (Vec<ImpactReason>, Confidence)> = BTreeMap::new();

        // Pour chaque changement
        for change in &changes.changes {
            // Règle 1 : relation directe sur l'élément exact
            let direct_rels = relations.relations_for_source(&change.element_id);
            for rel in direct_rels {
                if !doc_map.contains(&rel.target) {
                    continue;
                }
                let entry = doc_impacts
                    .entry(rel.target.clone())
                    .or_insert_with(|| (Vec::new(), rel.confidence));

                entry.0.push(ImpactReason {
                    change: change.clone(),
                    relation: rel.clone(),
                });

                // La confiance globale d'un document est le max des confiances de ses raisons
                if confidence_value(rel.confidence) > confidence_value(entry.1) {
                    entry.1 = rel.confidence;
                }
            }

            // Règle 2 : si c'est un élément de code (pas File), vérifier s'il existe une relation de niveau File
            if change.element_kind != SourceElementKind::File {
                let file_element_id = ElementId::for_file(&change.file);
                let file_rels = relations.relations_for_source(&file_element_id);
                for rel in file_rels {
                    if !doc_map.contains(&rel.target) {
                        continue;
                    }
                    let downgraded = rel.confidence.downgrade();
                    let entry = doc_impacts
                        .entry(rel.target.clone())
                        .or_insert_with(|| (Vec::new(), downgraded));

                    entry.0.push(ImpactReason {
                        change: change.clone(),
                        relation: rel.clone(),
                    });

                    if confidence_value(downgraded) > confidence_value(entry.1) {
                        entry.1 = downgraded;
                    }
                }
            }
        }

        let mut impacts: Vec<Impact> = doc_impacts
            .into_iter()
            .map(|(document, (reasons, confidence))| Impact {
                document,
                reasons,
                confidence,
            })
            .collect();

        impacts.sort_by(|a, b| a.document.cmp(&b.document));

        impacts
    }
}

fn confidence_value(c: Confidence) -> u8 {
    match c {
        Confidence::High => 3,
        Confidence::Medium => 2,
        Confidence::Low => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::change::{Change, ChangeKind};
    use crate::domain::document::{Document, DocumentId};
    use crate::domain::relation::DocumentationRelation;
    use std::path::PathBuf;

    #[test]
    fn test_no_changes_produces_no_impacts() {
        let changes = ChangeSet {
            changes: vec![],
            unchanged_files: vec!["src/main.rs".to_string()],
        };
        let relations = RelationSet::new(vec![]);
        let documents = vec![Document::new(
            "doc.md",
            Some("Doc".to_string()),
            PathBuf::from("doc.md"),
        )];

        let impacts = ImpactAnalyzer::analyze(&changes, &relations, &documents);
        assert!(impacts.is_empty());
    }

    #[test]
    fn test_direct_relation_impact() {
        let change = Change {
            kind: ChangeKind::Modified,
            element_id: ElementId::for_symbol("src/auth.py", "login"),
            element_kind: SourceElementKind::Function,
            file: "src/auth.py".to_string(),
            reason: "login was modified.".to_string(),
        };
        let changes = ChangeSet {
            changes: vec![change],
            unchanged_files: vec![],
        };
        let rel = DocumentationRelation::explicit(
            ElementId::for_symbol("src/auth.py", "login"),
            DocumentId("auth.md".to_string()),
        );
        let relations = RelationSet::new(vec![rel]);
        let documents = vec![Document::new(
            "auth.md",
            Some("Auth".to_string()),
            PathBuf::from("auth.md"),
        )];

        let impacts = ImpactAnalyzer::analyze(&changes, &relations, &documents);
        assert_eq!(impacts.len(), 1);
        assert_eq!(impacts[0].document, DocumentId("auth.md".to_string()));
        assert_eq!(impacts[0].confidence, Confidence::High);
    }

    #[test]
    fn test_file_level_relation_downgrades_confidence() {
        let change = Change {
            kind: ChangeKind::Modified,
            element_id: ElementId::for_symbol("src/auth.py", "login"),
            element_kind: SourceElementKind::Function,
            file: "src/auth.py".to_string(),
            reason: "login was modified.".to_string(),
        };
        let changes = ChangeSet {
            changes: vec![change],
            unchanged_files: vec![],
        };
        let rel = DocumentationRelation::explicit(
            ElementId::for_file("src/auth.py"),
            DocumentId("auth.md".to_string()),
        );
        let relations = RelationSet::new(vec![rel]);
        let documents = vec![Document::new(
            "auth.md",
            Some("Auth".to_string()),
            PathBuf::from("auth.md"),
        )];

        let impacts = ImpactAnalyzer::analyze(&changes, &relations, &documents);
        assert_eq!(impacts.len(), 1);
        assert_eq!(impacts[0].confidence, Confidence::Medium);
    }
}

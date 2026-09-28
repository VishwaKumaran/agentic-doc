//! Couverture documentaire dans le domaine.

use crate::domain::document::Document;
use crate::domain::relation::{RelationSet, RelationStatus};
use crate::domain::source::{ElementId, SourceElementKind, SourceModel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageCategory {
    Direct,
    ViaFile,
    Undocumented,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageEntry {
    pub element: ElementId,
    pub kind: SourceElementKind,
    pub category: CoverageCategory,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coverage {
    pub total: usize,
    pub direct: usize,
    pub via_file: usize,
    pub undocumented: usize,
    pub rate: f64,
    pub direct_entries: Vec<CoverageEntry>,
    pub via_file_entries: Vec<CoverageEntry>,
    pub undocumented_entries: Vec<CoverageEntry>,
    pub unreferenced_documents: Vec<String>,
}

pub struct CoverageCalculator;

impl CoverageCalculator {
    pub fn calculate(
        model: &SourceModel,
        relations: &RelationSet,
        documents: &[Document],
    ) -> Coverage {
        // Collecter les sources de relations validées
        let validated_relations: Vec<_> = relations
            .relations
            .iter()
            .filter(|r| r.status == RelationStatus::Validated)
            .collect();

        let mut direct_sources = std::collections::HashSet::new();
        let mut file_sources = std::collections::HashSet::new();
        let mut referenced_targets = std::collections::HashSet::new();

        for rel in &validated_relations {
            direct_sources.insert(&rel.source);
            referenced_targets.insert(&rel.target);
            let (file, name) = rel.source.parse();
            if name.is_none() {
                file_sources.insert(file);
            }
        }

        let mut direct_entries = Vec::new();
        let mut via_file_entries = Vec::new();
        let mut undocumented_entries = Vec::new();

        // Parcourir tous les éléments (fichiers + symboles)
        let all_elements = model.files.iter().chain(model.elements.iter());

        for elem in all_elements {
            let category = if direct_sources.contains(&elem.id) {
                CoverageCategory::Direct
            } else if elem.kind != SourceElementKind::File
                && file_sources.contains(elem.file.as_str())
            {
                CoverageCategory::ViaFile
            } else {
                CoverageCategory::Undocumented
            };

            let entry = CoverageEntry {
                element: elem.id.clone(),
                kind: elem.kind,
                category,
                file: elem.file.clone(),
            };

            match category {
                CoverageCategory::Direct => direct_sources_push(&mut direct_entries, entry),
                CoverageCategory::ViaFile => via_file_entries.push(entry),
                CoverageCategory::Undocumented => undocumented_entries.push(entry),
            }
        }

        // Tri déterministe par (file, element)
        let sort_entries = |entries: &mut Vec<CoverageEntry>| {
            entries.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.element.cmp(&b.element)));
        };
        sort_entries(&mut direct_entries);
        sort_entries(&mut via_file_entries);
        sort_entries(&mut undocumented_entries);

        let total = direct_entries.len() + via_file_entries.len() + undocumented_entries.len();
        let direct = direct_entries.len();
        let via_file = via_file_entries.len();
        let undocumented = undocumented_entries.len();

        let rate = if total == 0 {
            0.0
        } else {
            let r = (direct + via_file) as f64 / total as f64;
            (r * 100.0).round() / 100.0
        };

        // Documents non référencés
        let mut unreferenced_documents: Vec<String> = documents
            .iter()
            .map(|d| d.id.0.clone())
            .filter(|doc_id| !referenced_targets.iter().any(|t| t.0 == *doc_id))
            .collect();
        unreferenced_documents.sort();

        Coverage {
            total,
            direct,
            via_file,
            undocumented,
            rate,
            direct_entries,
            via_file_entries,
            undocumented_entries,
            unreferenced_documents,
        }
    }
}

fn direct_sources_push(vec: &mut Vec<CoverageEntry>, entry: CoverageEntry) {
    vec.push(entry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::document::{Document, DocumentId};
    use crate::domain::relation::{Confidence, DocumentationRelation, RelationOrigin};
    use crate::domain::source::{SourceElement, SourceElementKind};

    #[test]
    fn test_coverage_calculation_rules() {
        let file_elem = SourceElement {
            id: ElementId::for_file("src/auth.py"),
            kind: SourceElementKind::File,
            name: "src/auth.py".to_string(),
            file: "src/auth.py".to_string(),
            line_start: 1,
            line_end: 10,
            content_hash: 1,
        };
        let func1 = SourceElement {
            id: ElementId::for_symbol("src/auth.py", "login"),
            kind: SourceElementKind::Function,
            name: "login".to_string(),
            file: "src/auth.py".to_string(),
            line_start: 2,
            line_end: 5,
            content_hash: 2,
        };
        let func2 = SourceElement {
            id: ElementId::for_symbol("src/auth.py", "logout"),
            kind: SourceElementKind::Function,
            name: "logout".to_string(),
            file: "src/auth.py".to_string(),
            line_start: 6,
            line_end: 9,
            content_hash: 3,
        };

        let model = SourceModel::new("proj".to_string(), vec![file_elem], vec![func1, func2]);

        let doc1 = Document {
            id: DocumentId("auth.md".to_string()),
            title: Some("Auth".to_string()),
            path: std::path::PathBuf::from("auth.md"),
        };
        let doc2 = Document {
            id: DocumentId("other.md".to_string()),
            title: None,
            path: std::path::PathBuf::from("other.md"),
        };

        // Case 1: Direct relation to login, file relation to src/auth.py for file level, etc.
        let rel_direct = DocumentationRelation {
            source: ElementId::for_symbol("src/auth.py", "login"),
            target: DocumentId("auth.md".to_string()),
            origin: RelationOrigin::Explicit,
            status: RelationStatus::Validated,
            confidence: Confidence::High,
        };

        let rel_file = DocumentationRelation {
            source: ElementId::for_file("src/auth.py"),
            target: DocumentId("auth.md".to_string()),
            origin: RelationOrigin::Explicit,
            status: RelationStatus::Validated,
            confidence: Confidence::High,
        };

        let rel_set = RelationSet::new(vec![rel_direct, rel_file]);

        let coverage = CoverageCalculator::calculate(&model, &rel_set, &[doc1, doc2]);

        assert_eq!(coverage.total, 3);
        assert_eq!(coverage.direct, 2); // File element + login function
        assert_eq!(coverage.via_file, 1); // logout function is covered via_file
        assert_eq!(coverage.undocumented, 0);
        assert_eq!(coverage.rate, 1.0);
        assert_eq!(coverage.unreferenced_documents, vec!["other.md"]);
    }
}

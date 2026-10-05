//! Éléments sources et modèle source du projet.

use crate::domain::error::DomainError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceElementKind {
    File,
    Class,
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ElementId(pub String);

impl ElementId {
    pub fn for_file(path: &str) -> Self {
        Self(path.to_string())
    }

    pub fn for_symbol(path: &str, name: &str) -> Self {
        Self(format!("{}::{}", path, name))
    }

    pub fn parse(&self) -> (&str, Option<&str>) {
        if let Some((file, name)) = self.0.split_once("::") {
            (file, Some(name))
        } else {
            (&self.0, None)
        }
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ElementId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Err(DomainError::InvalidElementId(
                "Identifiant vide".to_string(),
            ));
        }
        Ok(Self(s.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceElement {
    pub id: ElementId,
    pub kind: SourceElementKind,
    pub name: String,
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub content_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceModel {
    pub project_id: String,
    pub files: Vec<SourceElement>,
    pub elements: Vec<SourceElement>,
}

impl SourceModel {
    pub fn new(
        project_id: String,
        mut files: Vec<SourceElement>,
        mut elements: Vec<SourceElement>,
    ) -> Self {
        files.sort_by(|a, b| a.file.cmp(&b.file));
        elements.sort_by(|a, b| a.id.cmp(&b.id));

        Self {
            project_id,
            files,
            elements,
        }
    }

    pub fn element(&self, id: &ElementId) -> Option<&SourceElement> {
        if let Ok(idx) = self.elements.binary_search_by(|e| e.id.cmp(id)) {
            Some(&self.elements[idx])
        } else {
            self.files.iter().find(|f| f.id == *id)
        }
    }

    pub fn elements_of_file(&self, path: &str) -> Vec<&SourceElement> {
        let mut res: Vec<&SourceElement> =
            self.elements.iter().filter(|e| e.file == path).collect();
        res.sort_by(|a, b| a.id.cmp(&b.id));
        res
    }

    /// Retourne `true` si l'analyse n'a trouvé aucun fichier source.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_id_parsing() {
        let id_file = ElementId::for_file("src/main.rs");
        assert_eq!(id_file.parse(), ("src/main.rs", None));

        let id_sym = ElementId::for_symbol("src/lib.rs", "my_func");
        assert_eq!(id_sym.parse(), ("src/lib.rs", Some("my_func")));
    }

    #[test]
    fn test_source_model_sorting_and_lookup() {
        let f1 = SourceElement {
            id: ElementId::for_file("b.rs"),
            kind: SourceElementKind::File,
            name: "b.rs".to_string(),
            file: "b.rs".to_string(),
            line_start: 1,
            line_end: 10,
            content_hash: 123,
        };
        let f2 = SourceElement {
            id: ElementId::for_file("a.rs"),
            kind: SourceElementKind::File,
            name: "a.rs".to_string(),
            file: "a.rs".to_string(),
            line_start: 1,
            line_end: 10,
            content_hash: 456,
        };

        let e1 = SourceElement {
            id: ElementId::for_symbol("a.rs", "foo"),
            kind: SourceElementKind::Function,
            name: "foo".to_string(),
            file: "a.rs".to_string(),
            line_start: 2,
            line_end: 5,
            content_hash: 789,
        };

        let model = SourceModel::new("p1".to_string(), vec![f1, f2], vec![e1.clone()]);
        assert_eq!(model.files[0].file, "a.rs");
        assert_eq!(model.files[1].file, "b.rs");

        assert_eq!(
            model.element(&ElementId::for_symbol("a.rs", "foo")),
            Some(&e1)
        );
        assert_eq!(model.elements_of_file("a.rs"), vec![&e1]);
    }
}

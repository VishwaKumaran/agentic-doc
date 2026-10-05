//! Détection par extension et analyse fusionnée des langages pris en charge.

use crate::analysis::analyzer::{AnalysisError, ProjectAnalyzer};
use crate::analysis::parser::SourceParser;
use crate::analysis::python::parser::PythonParser;
use crate::analysis::rust::parser::RustParser;
use crate::analysis::walk::should_ignore;
use crate::domain::hash::fnv1a_64;
use crate::domain::project::Project;
use crate::domain::source::{ElementId, SourceElement, SourceElementKind, SourceModel};
use std::fs;
use walkdir::WalkDir;

pub struct MultiLanguageAnalyzer {
    parsers: Vec<Box<dyn SourceParser>>,
}
impl MultiLanguageAnalyzer {
    pub fn new() -> Self {
        Self {
            parsers: vec![Box::new(PythonParser), Box::new(RustParser)],
        }
    }

    pub fn with_parsers(parsers: Vec<Box<dyn SourceParser>>) -> Self {
        Self { parsers }
    }
}
impl Default for MultiLanguageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectAnalyzer for MultiLanguageAnalyzer {
    fn analyze(&self, project: &Project) -> Result<SourceModel, AnalysisError> {
        if !project.root.exists() {
            return Err(AnalysisError::RootNotFound(
                project.root.to_string_lossy().to_string(),
            ));
        }
        let mut files = Vec::new();
        let mut elements = Vec::new();
        for entry in WalkDir::new(&project.root)
            .into_iter()
            .filter_entry(|e| !should_ignore(e.path(), &project.root))
        {
            let entry = entry.map_err(AnalysisError::WalkDir)?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            let Some(parser) = self
                .parsers
                .iter()
                .find(|p| p.extensions().contains(&extension))
            else {
                continue;
            };
            let relative = path
                .strip_prefix(&project.root)
                .map_err(|_| AnalysisError::UnreadableFile(path.to_string_lossy().to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            let content = fs::read_to_string(path)
                .map_err(|_| AnalysisError::UnreadableFile(path.to_string_lossy().to_string()))?;
            files.push(file_element(&relative, &content));
            elements.extend(parser.parse(&relative, &content));
        }
        Ok(SourceModel::new(project.id.clone(), files, elements))
    }
}

pub fn file_element(path: &str, content: &str) -> SourceElement {
    SourceElement {
        id: ElementId::for_file(path),
        kind: SourceElementKind::File,
        name: path.to_string(),
        file: path.to_string(),
        line_start: 1,
        line_end: content.lines().count().max(1),
        content_hash: fnv1a_64(content.as_bytes()),
    }
}

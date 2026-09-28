//! Adaptateur d'analyse pour les projets Python.

pub mod scanner;

use crate::analysis::analyzer::{AnalysisError, ProjectAnalyzer};
use crate::domain::project::Project;
use crate::domain::source::SourceModel;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct PythonAnalyzer;

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectAnalyzer for PythonAnalyzer {
    fn analyze(&self, project: &Project) -> Result<SourceModel, AnalysisError> {
        if !project.root.exists() {
            return Err(AnalysisError::RootNotFound(
                project.root.to_string_lossy().to_string(),
            ));
        }

        let mut files = Vec::new();
        let mut elements = Vec::new();

        let walker = WalkDir::new(&project.root).into_iter();

        for entry in walker.filter_entry(|e| !should_ignore(e.path(), &project.root)) {
            let entry = entry.map_err(AnalysisError::WalkDir)?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("py") {
                let relative = path
                    .strip_prefix(&project.root)
                    .map_err(|_| AnalysisError::UnreadableFile(path.to_string_lossy().to_string()))?
                    .to_string_lossy()
                    .to_string();

                let content = fs::read_to_string(path).map_err(|_| {
                    AnalysisError::UnreadableFile(path.to_string_lossy().to_string())
                })?;

                let (file_elem, code_elems) = scanner::scan_python_content(&relative, &content);
                files.push(file_elem);
                elements.extend(code_elems);
            }
        }

        Ok(SourceModel::new(project.id.clone(), files, elements))
    }
}

fn should_ignore(path: &Path, root: &Path) -> bool {
    if path == root {
        return false;
    }

    let name = match path.file_name().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return false,
    };

    if name.starts_with('.') {
        return true;
    }

    matches!(
        name,
        "__pycache__"
            | ".venv"
            | "venv"
            | "env"
            | "node_modules"
            | "target"
            | "build"
            | "dist"
            | ".agentic-doc"
            | ".git"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::source::SourceElementKind;

    #[test]
    fn test_python_scanner_extraction() {
        let py_code = r#"
class AuthService:
    def login(self):
        pass

def global_fn():
    pass
"#;
        let (file_elem, elements) = scanner::scan_python_content("src/auth.py", py_code);
        assert_eq!(file_elem.file, "src/auth.py");
        assert_eq!(elements.len(), 3);

        assert_eq!(elements[0].name, "AuthService");
        assert_eq!(elements[0].kind, SourceElementKind::Class);

        assert_eq!(elements[1].name, "AuthService.login");
        assert_eq!(elements[1].kind, SourceElementKind::Method);

        assert_eq!(elements[2].name, "global_fn");
        assert_eq!(elements[2].kind, SourceElementKind::Function);
    }
}

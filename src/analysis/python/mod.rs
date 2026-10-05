//! Adaptateur d'analyse pour les projets Python.

pub mod parser;

use crate::analysis::analyzer::{AnalysisError, ProjectAnalyzer};
use crate::analysis::multilang::MultiLanguageAnalyzer;
use crate::analysis::parser::SourceParser;
use crate::domain::project::Project;
use crate::domain::source::SourceModel;

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
        MultiLanguageAnalyzer::with_parsers(vec![
            Box::new(parser::PythonParser) as Box<dyn SourceParser>
        ])
        .analyze(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::parser::SourceParser;
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
        let elements = parser::PythonParser.parse("src/auth.py", py_code);
        assert_eq!(elements.len(), 3);

        assert_eq!(elements[0].name, "AuthService");
        assert_eq!(elements[0].kind, SourceElementKind::Class);

        assert_eq!(elements[1].name, "AuthService::login");
        assert_eq!(elements[1].kind, SourceElementKind::Method);

        assert_eq!(elements[2].name, "global_fn");
        assert_eq!(elements[2].kind, SourceElementKind::Function);
    }
}

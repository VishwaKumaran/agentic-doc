//! Analyse Rust. Les `const`, `static`, alias de type, macros, `use` et `extern crate`
//! sont volontairement ignorés, ainsi que les closures et items locaux aux fonctions.

pub mod parser;

use crate::analysis::analyzer::{AnalysisError, ProjectAnalyzer};
use crate::analysis::multilang::MultiLanguageAnalyzer;
use crate::analysis::parser::SourceParser;
use crate::domain::project::Project;
use crate::domain::source::SourceModel;

pub struct RustAnalyzer;
impl RustAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl ProjectAnalyzer for RustAnalyzer {
    fn analyze(&self, project: &Project) -> Result<SourceModel, AnalysisError> {
        MultiLanguageAnalyzer::with_parsers(vec![
            Box::new(parser::RustParser) as Box<dyn SourceParser>
        ])
        .analyze(project)
    }
}

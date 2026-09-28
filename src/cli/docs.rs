//! Commande CLI docs.

use crate::application::docs::{AnalyzeDocumentationImpact, DocsError, DocsMode, DocsResult};
use crate::cli::output;
use std::env;
use std::path::PathBuf;

pub fn execute(
    path: Option<PathBuf>,
    coverage: bool,
    all: bool,
    json: bool,
) -> Result<(), (String, i32)> {
    if all && !coverage {
        return Err(("--all requires --coverage.".to_string(), 2));
    }

    let workspace =
        env::current_dir().map_err(|e| (format!("cannot determine current directory: {e}"), 1))?;

    let mode = if coverage {
        DocsMode::Coverage { all }
    } else {
        DocsMode::Impacts
    };

    match AnalyzeDocumentationImpact::execute(&workspace, path, mode) {
        Ok(result) => {
            // Avertissement « 0 fichier analysé » (cli.md §3, étape 18)
            let no_files = match &result {
                DocsResult::Impacts {
                    no_files_analyzed, ..
                } => *no_files_analyzed,
                DocsResult::Coverage {
                    no_files_analyzed, ..
                } => *no_files_analyzed,
            };
            if no_files {
                output::emit_empty_analysis_warning();
            }
            if json {
                println!("{}", output::format_docs_json(&result));
            } else {
                println!("{}", output::format_docs_text(&result));
            }
            Ok(())
        }
        Err(err) => {
            let msg = match err {
                DocsError::NotConfigured => "agentic-doc is not configured in this directory.\nHint: run 'agentic-doc setup <project-path>' first.".to_string(),
                DocsError::InvalidRelations(ref detail) => format!("invalid relations file: {detail}"),
                DocsError::ProjectNotFound(ref p) => format!("project not found: {p}"),
                DocsError::AnalysisFailed(ref d) => format!("failed to analyze project: {d}"),
                DocsError::Storage(e) => format!("Storage error: {e}"),
                DocsError::Io(e) => format!("I/O error: {e}"),
            };
            Err((msg, 1))
        }
    }
}

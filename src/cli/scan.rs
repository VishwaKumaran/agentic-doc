//! Commande CLI scan.

use crate::application::scan::{ScanError, ScanProject};
use crate::cli::output;
use std::env;
use std::path::PathBuf;

pub fn execute(path: Option<PathBuf>, json: bool) -> Result<(), (String, i32)> {
    let workspace =
        env::current_dir().map_err(|e| (format!("cannot determine current directory: {e}"), 1))?;

    match ScanProject::execute(&workspace, path) {
        Ok(result) => {
            // Avertissement « 0 fichier analysé » (cli.md §3, étape 18)
            if result.no_files_analyzed {
                output::emit_empty_analysis_warning();
            }
            if json {
                println!("{}", output::format_scan_json(&result));
            } else {
                println!("{}", output::format_scan_text(&result));
            }
            Ok(())
        }
        Err(err) => {
            let msg = match err {
                ScanError::NotConfigured => "agentic-doc is not configured in this directory.\nHint: run 'agentic-doc setup <project-path>' first.".to_string(),
                ScanError::ProjectNotFound(ref p) => format!("project not found: {p}"),
                ScanError::AnalysisFailed(ref d) => format!("failed to analyze project: {d}"),
                ScanError::Storage(e) => format!("Storage error: {e}"),
                ScanError::Io(e) => format!("I/O error: {e}"),
            };
            Err((msg, 1))
        }
    }
}

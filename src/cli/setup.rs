//! Commande CLI setup.

use crate::application::setup::{SetupError, SetupProject};
use crate::cli::output;
use std::env;
use std::path::PathBuf;

pub fn execute(project_path: PathBuf, force: bool, json: bool) -> Result<(), (String, i32)> {
    let workspace =
        env::current_dir().map_err(|e| (format!("cannot determine current directory: {e}"), 1))?;

    match SetupProject::execute(&workspace, &project_path, force) {
        Ok(result) => {
            if json {
                println!("{}", output::format_setup_json(&result));
            } else {
                println!("{}", output::format_setup_text(&result));
            }
            Ok(())
        }
        Err(err) => {
            let msg = match err {
                SetupError::ProjectNotFound(ref path) => format!("project not found: {path}"),
                SetupError::CannotTargetSelf => {
                    "project docs cannot be the documented project.".to_string()
                }
                SetupError::AlreadyConfigured => {
                    "already configured. Use --force to overwrite.".to_string()
                }
                SetupError::Io(e) => format!("I/O error: {e}"),
                SetupError::Storage(e) => format!("Storage error: {e}"),
            };
            Err((msg, 1))
        }
    }
}

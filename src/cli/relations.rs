//! Commande CLI relations.

use crate::application::relations::{
    AddRelation, CheckRelations, ListRelations, RelationsError, RemoveRelation,
};
use crate::cli::output;
use crate::cli::RelationsCommands;
use crate::domain::document::DocumentId;
use crate::domain::source::ElementId;
use std::env;

pub fn execute(command: RelationsCommands, json: bool) -> Result<(), (String, i32)> {
    let workspace =
        env::current_dir().map_err(|e| (format!("cannot determine current directory: {e}"), 1))?;

    match command {
        RelationsCommands::List { path } => match ListRelations::execute(&workspace, path) {
            Ok(res) => {
                if res.no_files_analyzed {
                    output::emit_empty_analysis_warning();
                }
                if json {
                    println!("{}", output::format_relations_list_json(&res));
                } else {
                    println!("{}", output::format_relations_list_text(&res));
                }
                Ok(())
            }
            Err(err) => Err(handle_error(err)),
        },
        RelationsCommands::Check { path } => match CheckRelations::execute(&workspace, path) {
            Ok(res) => {
                if res.no_files_analyzed {
                    output::emit_empty_analysis_warning();
                }
                if json {
                    println!("{}", output::format_relations_check_json(&res));
                } else {
                    println!("{}", output::format_relations_check_text(&res));
                }
                Ok(())
            }
            Err(err) => Err(handle_error(err)),
        },
        RelationsCommands::Add {
            source,
            target,
            confidence,
            dry_run,
            path,
        } => {
            let src_id = ElementId::for_file(&source);
            let target_id = DocumentId(target);
            let conf = confidence.map(Into::into);

            match AddRelation::execute(&workspace, path, src_id, target_id, conf, dry_run) {
                Ok(res) => {
                    if res.no_files_analyzed {
                        output::emit_empty_analysis_warning();
                    }
                    if json {
                        println!("{}", output::format_relations_add_json(&res));
                    } else {
                        println!("{}", output::format_relations_add_text(&res));
                    }
                    Ok(())
                }
                Err(err) => Err(handle_error(err)),
            }
        }
        RelationsCommands::Remove {
            source,
            target,
            dry_run,
            path,
        } => {
            let src_id = ElementId::for_file(&source);
            let target_id = DocumentId(target);

            match RemoveRelation::execute(&workspace, path, src_id, target_id, dry_run) {
                Ok(res) => {
                    if res.no_files_analyzed {
                        output::emit_empty_analysis_warning();
                    }
                    if !res.removed && !json {
                        eprintln!("Nothing to remove.");
                    }
                    if json {
                        println!("{}", output::format_relations_remove_json(&res));
                    } else if res.removed || res.dry_run {
                        println!("{}", output::format_relations_remove_text(&res));
                    }
                    Ok(())
                }
                Err(err) => Err(handle_error(err)),
            }
        }
    }
}

fn handle_error(err: RelationsError) -> (String, i32) {
    let msg = match err {
        RelationsError::NotConfigured => "agentic-doc is not configured in this directory.\nHint: run 'agentic-doc setup <project-path>' first.".to_string(),
        RelationsError::InvalidRelations(ref detail) => format!("invalid relations file: {detail}"),
        RelationsError::SourceNotFound(ref src) => format!("relation source not found: {src}"),
        RelationsError::TargetNotFound(ref target) => format!("relation target not found: {target}"),
        RelationsError::ProjectNotFound(ref p) => format!("project not found: {p}"),
        RelationsError::AnalysisFailed(ref d) => format!("failed to analyze project: {d}"),
        RelationsError::Storage(e) => format!("Storage error: {e}"),
        RelationsError::Io(e) => format!("I/O error: {e}"),
    };
    (msg, 1)
}

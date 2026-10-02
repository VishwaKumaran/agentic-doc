//! Point d'entrée principal du binaire agentic-doc.

pub mod analysis;
pub mod application;
pub mod cli;
pub mod documentation;
pub mod domain;
pub mod impact;
pub mod infrastructure;

use clap::Parser;
use cli::{Cli, Commands};
use std::process;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Setup {
            project_path,
            force,
        } => cli::setup::execute(project_path, force, cli.json),
        Commands::Scan { path } => cli::scan::execute(path, cli.json),
        Commands::Status { path } => cli::status::execute(path, cli.json),
        Commands::Docs {
            path,
            coverage,
            all,
        } => cli::docs::execute(path, coverage, all, cli.json),
        Commands::Relations { command } => cli::relations::execute(command, cli.json),
        Commands::Skills { command } => match cli::skills::execute(command, cli.json) {
            // `skills` est un alias de `npx skills` : le code de sortie de l'enfant
            // est propagé tel quel, sans message supplémentaire.
            Ok(0) => Ok(()),
            Ok(code) => process::exit(code),
            Err(err) => Err(err),
        },
    };

    if let Err((msg, exit_code)) = result {
        eprintln!("Error: {msg}");
        process::exit(exit_code);
    }
}

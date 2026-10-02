//! Commande CLI `skills` : alias mince vers l'outil standard `npx skills`.
//!
//! La commande n'implémente aucune installation : elle délègue à `npx skills add`
//! sur la source publiée de ce dépôt et propage le code de sortie de l'enfant.

use crate::cli::SkillsCommands;
use std::process::Command;

/// Source publiée du skill distribué par ce dépôt.
const SKILL_SOURCE: &str = "VishwaKumaran/agentic-doc";

/// Exécute `npx -y skills add VishwaKumaran/agentic-doc [ARGS]…`.
///
/// `Ok(code)` porte le code de sortie de `npx` — y compris non nul, propagé tel quel.
/// `Err` ne couvre que les échecs propres au binaire (npx introuvable, signal).
pub fn execute(command: SkillsCommands, json: bool) -> Result<i32, (String, i32)> {
    let SkillsCommands::Install { args } = command;
    execute_install(&args, json)
}

fn execute_install(passthrough: &[String], json: bool) -> Result<i32, (String, i32)> {
    let mut args: Vec<String> = vec![
        "-y".to_string(),
        "skills".to_string(),
        "add".to_string(),
        SKILL_SOURCE.to_string(),
    ];

    // `--json` est un flag global de `agentic-doc` : clap le consomme quelle que soit
    // sa position, donc on le ré-émet vers npx pour qu'il ne disparaisse pas.
    if json {
        args.push("--json".to_string());
    }
    args.extend(passthrough.iter().cloned());

    match Command::new("npx").args(&args).status() {
        Ok(status) => match status.code() {
            Some(code) => Ok(code),
            None => Err(("npx was terminated by a signal.".to_string(), 1)),
        },
        Err(_) => Err((npx_not_found_message(), 1)),
    }
}

fn npx_not_found_message() -> String {
    format!(
        "npx not found. Install Node.js (https://nodejs.org) or run 'npx skills add {}' manually.",
        SKILL_SOURCE
    )
}

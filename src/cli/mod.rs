//! Couche d'interface utilisateur CLI.

pub mod docs;
pub mod output;
pub mod relations;
pub mod scan;
pub mod setup;
pub mod status;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agentic-doc", author, version, about = "Outil de maintien de la cohérence entre code et documentation", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Format de sortie JSON
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialise le projet docs et le lie au projet code
    Setup {
        /// Chemin du projet code à documenter
        project_path: PathBuf,

        /// Écrase la configuration existante
        #[arg(long)]
        force: bool,
    },
    /// Analyse le projet et enregistre un snapshot de son état
    Scan {
        /// Chemin du projet code (facultatif si configuré)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Affiche les changements détectés depuis le dernier snapshot
    Status {
        /// Chemin du projet code (facultatif si configuré)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Analyse l'impact des changements sur la documentation ou la couverture
    Docs {
        /// Chemin du projet code (facultatif si configuré)
        #[arg(long)]
        path: Option<PathBuf>,

        /// Affiche la couverture documentaire au lieu de l'analyse d'impact
        #[arg(long)]
        coverage: bool,

        /// Avec --coverage : affiche les trois catégories d'éléments au lieu des seuls non documentés
        #[arg(long)]
        all: bool,
    },
    /// Gère les relations déclarées entre le projet code et la documentation
    Relations {
        #[command(subcommand)]
        command: RelationsCommands,
    },
}

#[derive(Subcommand)]
pub enum RelationsCommands {
    /// Affiche les relations déclarées
    List {
        /// Chemin du projet code (facultatif si configuré)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Déclare une relation entre un élément source et un document
    Add {
        /// Identifiant de l'élément source
        source: String,

        /// Chemin du document cible
        target: String,

        /// Niveau de confiance
        #[arg(long, value_enum)]
        confidence: Option<ConfidenceArg>,

        /// Valide et affiche le résultat sans écrire
        #[arg(long)]
        dry_run: bool,

        /// Chemin du projet code (facultatif si configuré)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Supprime une relation déclarée
    Remove {
        /// Identifiant de l'élément source
        source: String,

        /// Chemin du document cible
        target: String,

        /// Valide et affiche le résultat sans écrire
        #[arg(long)]
        dry_run: bool,

        /// Chemin du projet code (facultatif si configuré)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Effectue un audit des relations déclarées
    Check {
        /// Chemin du projet code (facultatif si configuré)
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfidenceArg {
    High,
    Medium,
    Low,
}

impl From<ConfidenceArg> for crate::domain::relation::Confidence {
    fn from(arg: ConfidenceArg) -> Self {
        match arg {
            ConfidenceArg::High => crate::domain::relation::Confidence::High,
            ConfidenceArg::Medium => crate::domain::relation::Confidence::Medium,
            ConfidenceArg::Low => crate::domain::relation::Confidence::Low,
        }
    }
}

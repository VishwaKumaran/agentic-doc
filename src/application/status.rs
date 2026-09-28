//! Use case GetProjectStatus : calcul des changements entre le dernier snapshot et l'état courant.

use crate::analysis::analyzer::{AnalysisError, ProjectAnalyzer};
use crate::analysis::python::PythonAnalyzer;
use crate::domain::change::{ChangeDetector, ChangeSet};
use crate::domain::project::Project;
use crate::domain::snapshot::Snapshot;
use crate::infrastructure::config::WorkspaceConfig;
use crate::infrastructure::filesystem::StorageError;
use crate::infrastructure::snapshots::{FileSystemSnapshotRepository, SnapshotRepository};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("agentic-doc is not configured in this directory.\nHint: run 'agentic-doc setup <project-path>' first.")]
    NotConfigured,

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("failed to analyze project: {0}")]
    AnalysisFailed(String),

    #[error("Erreur de stockage : {0}")]
    Storage(#[from] StorageError),

    #[error("Erreur d'E/S : {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct StatusResult {
    pub project: Project,
    /// Snapshot précédent, ou `None` au premier run (aucun snapshot enregistré).
    pub previous_snapshot: Option<Snapshot>,
    pub changeset: ChangeSet,
    /// `true` si aucun fichier source n'a été trouvé lors de l'analyse courante.
    pub no_files_analyzed: bool,
}

pub struct GetProjectStatus;

impl GetProjectStatus {
    pub fn execute(
        workspace: &Path,
        project_path_override: Option<PathBuf>,
    ) -> Result<StatusResult, StatusError> {
        let canonical_workspace = workspace.canonicalize().map_err(StatusError::Io)?;

        let config = WorkspaceConfig::load(&canonical_workspace).map_err(|e| match e {
            StorageError::ConfigNotFound => StatusError::NotConfigured,
            other => StatusError::Storage(other),
        })?;

        let project_target = match project_path_override {
            Some(p) => {
                if p.is_relative() {
                    canonical_workspace.join(p)
                } else {
                    p
                }
            }
            None => canonical_workspace.join(&config.project_root),
        };

        let canonical_project = project_target.canonicalize().map_err(|_| {
            StatusError::ProjectNotFound(project_target.to_string_lossy().to_string())
        })?;

        let project = Project::new(&canonical_project)
            .map_err(|e| StatusError::AnalysisFailed(e.to_string()))?;

        let repo = FileSystemSnapshotRepository::new(canonical_workspace);
        // L'absence de snapshot n'est plus une erreur : c'est le premier run.
        let previous_snapshot: Option<Snapshot> = repo.get_latest(&project)?;

        let analyzer = PythonAnalyzer::new();
        let current_model = analyzer.analyze(&project).map_err(|e| match e {
            AnalysisError::RootNotFound(p) => StatusError::ProjectNotFound(p),
            other => StatusError::AnalysisFailed(other.to_string()),
        })?;

        let no_files_analyzed = current_model.is_empty();
        let changeset = ChangeDetector::detect(previous_snapshot.as_ref(), &current_model);

        Ok(StatusResult {
            project,
            previous_snapshot,
            changeset,
            no_files_analyzed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::scan::ScanProject;
    use crate::application::setup::SetupProject;
    use crate::domain::change::ChangeKind;
    use tempfile::tempdir;

    #[test]
    fn test_status_sans_snapshot_premier_run_succes() {
        // Au premier run, sans snapshot, status doit réussir et rapporter tous les éléments comme Added.
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("app.py"), "def start(): pass").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();

        let res = GetProjectStatus::execute(ws.path(), None).unwrap();
        assert!(res.previous_snapshot.is_none());
        // Tous les changements doivent être Added
        assert!(!res.changeset.changes.is_empty());
        assert!(res
            .changeset
            .changes
            .iter()
            .all(|c| c.kind == ChangeKind::Added));
    }

    #[test]
    fn test_status_after_scan_shows_no_changes() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("app.py"), "def start(): pass").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();
        ScanProject::execute(ws.path(), None).unwrap();

        let res = GetProjectStatus::execute(ws.path(), None).unwrap();
        assert!(res.previous_snapshot.is_some());
        assert!(res.changeset.changes.is_empty());
        assert_eq!(res.changeset.unchanged_files, vec!["app.py"]);
    }
}

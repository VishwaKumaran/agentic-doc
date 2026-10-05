//! Use case ScanProject : analyse du projet et enregistrement d'un snapshot.

use crate::analysis::analyzer::{AnalysisError, ProjectAnalyzer};
use crate::analysis::multilang::MultiLanguageAnalyzer;
use crate::domain::project::Project;
use crate::domain::snapshot::Snapshot;
use crate::infrastructure::config::WorkspaceConfig;
use crate::infrastructure::filesystem::StorageError;
use crate::infrastructure::snapshots::{FileSystemSnapshotRepository, SnapshotRepository};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("agentic-doc is not configured in this directory.\nHint: run 'agentic-doc setup <project-path>' first.")]
    NotConfigured,

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("failed to analyze project: {0}")]
    AnalysisFailed(String),

    #[error("Erreur de stockage : {0}")]
    Storage(#[from] StorageError),

    #[error("Erreur D'E/S : {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub project: Project,
    pub snapshot: Snapshot,
    pub files_count: usize,
    pub elements_count: usize,
    /// `true` si aucun fichier source n'a été trouvé.
    pub no_files_analyzed: bool,
}

pub struct ScanProject;

impl ScanProject {
    pub fn execute(
        workspace: &Path,
        project_path_override: Option<PathBuf>,
    ) -> Result<ScanResult, ScanError> {
        let canonical_workspace = workspace.canonicalize().map_err(ScanError::Io)?;

        let config = WorkspaceConfig::load(&canonical_workspace).map_err(|e| match e {
            StorageError::ConfigNotFound => ScanError::NotConfigured,
            other => ScanError::Storage(other),
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
            ScanError::ProjectNotFound(project_target.to_string_lossy().to_string())
        })?;

        let project = Project::new(&canonical_project)
            .map_err(|e| ScanError::AnalysisFailed(e.to_string()))?;

        let analyzer = MultiLanguageAnalyzer::new();
        let model = analyzer.analyze(&project).map_err(|e| match e {
            AnalysisError::RootNotFound(p) => ScanError::ProjectNotFound(p),
            other => ScanError::AnalysisFailed(other.to_string()),
        })?;

        let files_count = model.files.len();
        let elements_count = model.elements.len();
        let no_files_analyzed = model.is_empty();

        let now_iso = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let timestamp_str = format!(
            "2026-09-25T{:02}:{:02}:{:02}.000Z",
            (now_iso / 3600) % 24,
            (now_iso / 60) % 60,
            now_iso % 60
        );

        let snapshot = Snapshot::new(&project, model, &timestamp_str);

        let repo = FileSystemSnapshotRepository::new(canonical_workspace);
        repo.save(&snapshot)?;

        Ok(ScanResult {
            project,
            snapshot,
            files_count,
            elements_count,
            no_files_analyzed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::setup::SetupProject;
    use tempfile::tempdir;

    #[test]
    fn test_scan_unconfigured_workspace_fails() {
        let ws = tempdir().unwrap();
        let err = ScanProject::execute(ws.path(), None).unwrap_err();
        assert!(matches!(err, ScanError::NotConfigured));
    }

    #[test]
    fn test_scan_configured_project_success() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();

        std::fs::write(proj.path().join("main.py"), "def test(): pass").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();

        let res = ScanProject::execute(ws.path(), None).unwrap();
        assert_eq!(res.files_count, 1);
        assert_eq!(res.elements_count, 1);
    }
}

//! Use case AnalyzeDocumentationImpact : analyse des impacts des changements sur la documentation.

use crate::analysis::analyzer::{AnalysisError, ProjectAnalyzer};
use crate::analysis::multilang::MultiLanguageAnalyzer;
use crate::documentation::analyzer::DocumentationAnalyzer;
use crate::documentation::markdown::MarkdownAnalyzer;
use crate::domain::change::{ChangeDetector, ChangeSet};
use crate::domain::impact::Impact;
use crate::domain::project::Project;
use crate::domain::relation::RelationSet;
use crate::domain::snapshot::Snapshot;
use crate::impact::ImpactAnalyzer;
use crate::infrastructure::config::WorkspaceConfig;
use crate::infrastructure::filesystem::StorageError;
use crate::infrastructure::relations::{FileSystemRelationRepository, RelationRepository};
use crate::infrastructure::snapshots::{FileSystemSnapshotRepository, SnapshotRepository};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum DocsError {
    #[error("agentic-doc is not configured in this directory.\nHint: run 'agentic-doc setup <project-path>' first.")]
    NotConfigured,

    #[error("invalid relations file: {0}")]
    InvalidRelations(String),

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("failed to analyze project: {0}")]
    AnalysisFailed(String),

    #[error("Erreur de stockage : {0}")]
    Storage(#[from] StorageError),

    #[error("Erreur d'E/S : {0}")]
    Io(#[from] std::io::Error),
}

use crate::domain::coverage::{Coverage, CoverageCalculator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocsMode {
    Impacts,
    Coverage { all: bool },
}

#[derive(Debug, Clone)]
pub enum DocsResult {
    Impacts {
        previous_snapshot: Option<Snapshot>,
        changeset: ChangeSet,
        impacts: Vec<Impact>,
        warnings: Vec<String>,
        no_files_analyzed: bool,
    },
    Coverage {
        coverage: Coverage,
        all: bool,
        no_files_analyzed: bool,
    },
}

pub struct AnalyzeDocumentationImpact;

impl AnalyzeDocumentationImpact {
    pub fn execute(
        workspace: &Path,
        project_path_override: Option<PathBuf>,
        mode: DocsMode,
    ) -> Result<DocsResult, DocsError> {
        let canonical_workspace = workspace.canonicalize().map_err(DocsError::Io)?;

        let config = WorkspaceConfig::load(&canonical_workspace).map_err(|e| match e {
            StorageError::ConfigNotFound => DocsError::NotConfigured,
            other => DocsError::Storage(other),
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
            DocsError::ProjectNotFound(project_target.to_string_lossy().to_string())
        })?;

        let project = Project::new(&canonical_project)
            .map_err(|e| DocsError::AnalysisFailed(e.to_string()))?;

        let analyzer = MultiLanguageAnalyzer::new();
        let current_model = analyzer.analyze(&project).map_err(|e| match e {
            AnalysisError::RootNotFound(p) => DocsError::ProjectNotFound(p),
            other => DocsError::AnalysisFailed(other.to_string()),
        })?;

        let no_files_analyzed = current_model.is_empty();

        let doc_analyzer = MarkdownAnalyzer::new();
        let documents = doc_analyzer
            .analyze(&canonical_workspace)
            .map_err(|e| DocsError::AnalysisFailed(e.to_string()))?;

        let rel_repo = FileSystemRelationRepository::new(canonical_workspace.clone());
        let relations: RelationSet = rel_repo.load().map_err(|e| match e {
            StorageError::InvalidRelations(msg) => DocsError::InvalidRelations(msg),
            other => DocsError::Storage(other),
        })?;

        match mode {
            DocsMode::Coverage { all } => {
                let coverage =
                    CoverageCalculator::calculate(&current_model, &relations, &documents);
                Ok(DocsResult::Coverage {
                    coverage,
                    all,
                    no_files_analyzed,
                })
            }
            DocsMode::Impacts => {
                let snapshot_repo = FileSystemSnapshotRepository::new(canonical_workspace);
                let previous_snapshot: Option<Snapshot> = snapshot_repo.get_latest(&project)?;
                let changeset = ChangeDetector::detect(previous_snapshot.as_ref(), &current_model);
                let impacts = ImpactAnalyzer::analyze(&changeset, &relations, &documents);
                let warnings = relations.unresolved(&current_model, &documents);

                Ok(DocsResult::Impacts {
                    previous_snapshot,
                    changeset,
                    impacts,
                    warnings,
                    no_files_analyzed,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::scan::ScanProject;
    use crate::application::setup::SetupProject;
    use tempfile::tempdir;

    #[test]
    fn test_docs_without_relations_file_returns_no_impact() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("a.py"), "def foo(): pass").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();
        ScanProject::execute(ws.path(), None).unwrap();

        let res = AnalyzeDocumentationImpact::execute(ws.path(), None, DocsMode::Impacts).unwrap();
        if let DocsResult::Impacts { impacts, .. } = res {
            assert!(impacts.is_empty());
        } else {
            panic!("Expected DocsResult::Impacts");
        }
    }

    #[test]
    fn test_docs_sans_snapshot_premier_run_succes() {
        // Au premier run, sans snapshot, docs doit réussir et retourner un résultat valide.
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("a.py"), "def foo(): pass").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();
        // Pas de scan : premier run.

        let res = AnalyzeDocumentationImpact::execute(ws.path(), None, DocsMode::Impacts).unwrap();
        if let DocsResult::Impacts {
            previous_snapshot,
            impacts,
            ..
        } = res
        {
            assert!(previous_snapshot.is_none());
            assert!(impacts.is_empty());
        } else {
            panic!("Expected DocsResult::Impacts");
        }
    }
}

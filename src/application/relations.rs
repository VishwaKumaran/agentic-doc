//! Use cases pour la gestion des relations de documentation.

use crate::analysis::analyzer::{AnalysisError, ProjectAnalyzer};
use crate::analysis::python::PythonAnalyzer;
use crate::documentation::analyzer::DocumentationAnalyzer;
use crate::documentation::markdown::MarkdownAnalyzer;
use crate::domain::document::DocumentId;
use crate::domain::project::Project;
use crate::domain::relation::{Confidence, DocumentationRelation, InsertOutcome, RelationSet};
use crate::domain::source::{ElementId, SourceModel};
use crate::infrastructure::config::WorkspaceConfig;
use crate::infrastructure::filesystem::StorageError;
use crate::infrastructure::relations::{FileSystemRelationRepository, RelationRepository};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum RelationsError {
    #[error("agentic-doc is not configured in this directory.\nHint: run 'agentic-doc setup <project-path>' first.")]
    NotConfigured,

    #[error("invalid relations file: {0}")]
    InvalidRelations(String),

    #[error("relation source not found: {0}")]
    SourceNotFound(String),

    #[error("relation target not found: {0}")]
    TargetNotFound(String),

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
pub struct ResolvedRelationInfo {
    pub relation: DocumentationRelation,
    pub resolution: RelationResolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationResolution {
    Ok,
    SourceNotFound,
    TargetNotFound,
    SourceAndTargetNotFound,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ListRelationsResult {
    pub relations: Vec<ResolvedRelationInfo>,
    pub no_files_analyzed: bool,
}

#[derive(Debug, Clone)]
pub struct CheckRelationsSummary {
    pub ok: usize,
    pub sources_not_found: usize,
    pub targets_not_found: usize,
    pub duplicates: usize,
}

#[derive(Debug, Clone)]
pub struct CheckRelationsResult {
    pub relations: Vec<ResolvedRelationInfo>,
    pub summary: CheckRelationsSummary,
    pub no_files_analyzed: bool,
}

#[derive(Debug, Clone)]
pub struct AddRelationResult {
    pub result: InsertOutcome,
    pub relation: DocumentationRelation,
    pub dry_run: bool,
    pub no_files_analyzed: bool,
}

#[derive(Debug, Clone)]
pub struct RemoveRelationResult {
    pub removed: bool,
    pub count: usize,
    pub source: ElementId,
    pub target: DocumentId,
    pub dry_run: bool,
    pub no_files_analyzed: bool,
}

pub struct ListRelations;
pub struct CheckRelations;
pub struct AddRelation;
pub struct RemoveRelation;

fn load_context(
    workspace: &Path,
    project_path_override: Option<PathBuf>,
) -> Result<
    (
        PathBuf,
        Project,
        SourceModel,
        Vec<crate::domain::document::Document>,
        RelationSet,
    ),
    RelationsError,
> {
    let canonical_workspace = workspace.canonicalize().map_err(RelationsError::Io)?;

    let config = WorkspaceConfig::load(&canonical_workspace).map_err(|e| match e {
        StorageError::ConfigNotFound => RelationsError::NotConfigured,
        other => RelationsError::Storage(other),
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
        RelationsError::ProjectNotFound(project_target.to_string_lossy().to_string())
    })?;

    let project = Project::new(&canonical_project)
        .map_err(|e| RelationsError::AnalysisFailed(e.to_string()))?;

    let py_analyzer = PythonAnalyzer::new();
    let current_model = py_analyzer.analyze(&project).map_err(|e| match e {
        AnalysisError::RootNotFound(p) => RelationsError::ProjectNotFound(p),
        other => RelationsError::AnalysisFailed(other.to_string()),
    })?;

    let doc_analyzer = MarkdownAnalyzer::new();
    let documents = doc_analyzer
        .analyze(&canonical_workspace)
        .map_err(|e| RelationsError::AnalysisFailed(e.to_string()))?;

    let rel_repo = FileSystemRelationRepository::new(canonical_workspace.clone());
    let relations = rel_repo.load().map_err(|e| match e {
        StorageError::InvalidRelations(msg) => RelationsError::InvalidRelations(msg),
        other => RelationsError::Storage(other),
    })?;

    Ok((
        canonical_workspace,
        project,
        current_model,
        documents,
        relations,
    ))
}

fn resolve_relation(
    rel: &DocumentationRelation,
    model: &SourceModel,
    documents: &[crate::domain::document::Document],
) -> RelationResolution {
    let known_elements: std::collections::HashSet<&str> = model
        .files
        .iter()
        .chain(model.elements.iter())
        .map(|e| e.id.0.as_str())
        .collect();
    let known_documents: std::collections::HashSet<&str> =
        documents.iter().map(|d| d.id.0.as_str()).collect();

    let source_ok = known_elements.contains(rel.source.0.as_str());
    let target_ok = known_documents.contains(rel.target.0.as_str());

    match (source_ok, target_ok) {
        (true, true) => RelationResolution::Ok,
        (false, true) => RelationResolution::SourceNotFound,
        (true, false) => RelationResolution::TargetNotFound,
        (false, false) => RelationResolution::SourceAndTargetNotFound,
    }
}

impl ListRelations {
    pub fn execute(
        workspace: &Path,
        project_path_override: Option<PathBuf>,
    ) -> Result<ListRelationsResult, RelationsError> {
        let (_ws, _proj, model, docs, rel_set) = load_context(workspace, project_path_override)?;
        let no_files_analyzed = model.is_empty();

        let resolved_relations = rel_set
            .relations
            .iter()
            .map(|r| ResolvedRelationInfo {
                relation: r.clone(),
                resolution: resolve_relation(r, &model, &docs),
            })
            .collect();

        Ok(ListRelationsResult {
            relations: resolved_relations,
            no_files_analyzed,
        })
    }
}

impl CheckRelations {
    pub fn execute(
        workspace: &Path,
        project_path_override: Option<PathBuf>,
    ) -> Result<CheckRelationsResult, RelationsError> {
        let (_ws, _proj, model, docs, rel_set) = load_context(workspace, project_path_override)?;
        let no_files_analyzed = model.is_empty();

        let mut ok = 0;
        let mut sources_not_found = 0;
        let mut targets_not_found = 0;

        let resolved_relations: Vec<_> = rel_set
            .relations
            .iter()
            .map(|r| {
                let res = resolve_relation(r, &model, &docs);
                match res {
                    RelationResolution::Ok => ok += 1,
                    RelationResolution::SourceNotFound => sources_not_found += 1,
                    RelationResolution::TargetNotFound => targets_not_found += 1,
                    RelationResolution::SourceAndTargetNotFound => {
                        sources_not_found += 1;
                        targets_not_found += 1;
                    }
                }
                ResolvedRelationInfo {
                    relation: r.clone(),
                    resolution: res,
                }
            })
            .collect();

        let duplicates_count = rel_set.duplicates().len();

        let summary = CheckRelationsSummary {
            ok,
            sources_not_found,
            targets_not_found,
            duplicates: duplicates_count,
        };

        Ok(CheckRelationsResult {
            relations: resolved_relations,
            summary,
            no_files_analyzed,
        })
    }
}

impl AddRelation {
    pub fn execute(
        workspace: &Path,
        project_path_override: Option<PathBuf>,
        source: ElementId,
        target: DocumentId,
        confidence: Option<Confidence>,
        dry_run: bool,
    ) -> Result<AddRelationResult, RelationsError> {
        let (ws, _proj, model, docs, mut rel_set) = load_context(workspace, project_path_override)?;
        let no_files_analyzed = model.is_empty();

        let conf = confidence.unwrap_or(Confidence::High);
        let candidate_rel = DocumentationRelation {
            source: source.clone(),
            target: target.clone(),
            origin: crate::domain::relation::RelationOrigin::Explicit,
            status: crate::domain::relation::RelationStatus::Validated,
            confidence: conf,
        };

        // Validation stricte contre l'état courant (cli.md §8 & §11 décision 29)
        let resolution = resolve_relation(&candidate_rel, &model, &docs);
        match resolution {
            RelationResolution::SourceNotFound => {
                return Err(RelationsError::SourceNotFound(source.0));
            }
            RelationResolution::TargetNotFound => {
                return Err(RelationsError::TargetNotFound(target.0));
            }
            RelationResolution::SourceAndTargetNotFound => {
                // Si la source et la cible sont toutes deux introuvables, émettre d'abord la source
                // (le CLI captera la première erreur typée et la CLI pourra afficher la seconde si besoin,
                // mais le use case retourne la première).
                return Err(RelationsError::SourceNotFound(source.0));
            }
            RelationResolution::Ok => {}
        }

        let outcome = rel_set.insert(candidate_rel.clone());

        if !dry_run {
            let repo = FileSystemRelationRepository::new(ws);
            repo.save(&rel_set)?;
        }

        Ok(AddRelationResult {
            result: outcome,
            relation: candidate_rel,
            dry_run,
            no_files_analyzed,
        })
    }
}

impl RemoveRelation {
    pub fn execute(
        workspace: &Path,
        project_path_override: Option<PathBuf>,
        source: ElementId,
        target: DocumentId,
        dry_run: bool,
    ) -> Result<RemoveRelationResult, RelationsError> {
        let (ws, _proj, model, _docs, mut rel_set) =
            load_context(workspace, project_path_override)?;
        let no_files_analyzed = model.is_empty();

        let count = rel_set.remove(&source, &target);
        let removed = count > 0;

        if !dry_run {
            let repo = FileSystemRelationRepository::new(ws);
            repo.save(&rel_set)?;
        }

        Ok(RemoveRelationResult {
            removed,
            count,
            source,
            target,
            dry_run,
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
    fn test_relations_unconfigured_fails() {
        let ws = tempdir().unwrap();
        let err = ListRelations::execute(ws.path(), None).unwrap_err();
        assert!(matches!(err, RelationsError::NotConfigured));
    }

    #[test]
    fn test_add_relation_unknown_source_fails_without_writing() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("main.py"), "def run(): pass").unwrap();
        std::fs::write(ws.path().join("doc.md"), "# Doc").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();

        let err = AddRelation::execute(
            ws.path(),
            None,
            ElementId::for_symbol("main.py", "unknown"),
            DocumentId("doc.md".to_string()),
            None,
            false,
        )
        .unwrap_err();

        assert!(matches!(err, RelationsError::SourceNotFound(_)));
        assert!(!ws.path().join(".agentic-doc/relations.json").exists());
    }

    #[test]
    fn test_add_relation_unknown_target_fails_without_writing() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("main.py"), "def run(): pass").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();

        let err = AddRelation::execute(
            ws.path(),
            None,
            ElementId::for_symbol("main.py", "run"),
            DocumentId("unknown.md".to_string()),
            None,
            false,
        )
        .unwrap_err();

        assert!(matches!(err, RelationsError::TargetNotFound(_)));
        assert!(!ws.path().join(".agentic-doc/relations.json").exists());
    }

    #[test]
    fn test_add_relation_success_and_idempotence() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("main.py"), "def run(): pass").unwrap();
        std::fs::write(ws.path().join("doc.md"), "# Doc").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();

        let res1 = AddRelation::execute(
            ws.path(),
            None,
            ElementId::for_symbol("main.py", "run"),
            DocumentId("doc.md".to_string()),
            None,
            false,
        )
        .unwrap();

        assert_eq!(res1.result, InsertOutcome::Added);
        assert!(ws.path().join(".agentic-doc/relations.json").exists());

        let res2 = AddRelation::execute(
            ws.path(),
            None,
            ElementId::for_symbol("main.py", "run"),
            DocumentId("doc.md".to_string()),
            None,
            false,
        )
        .unwrap();

        assert_eq!(res2.result, InsertOutcome::Unchanged);
    }

    #[test]
    fn test_add_relation_update_confidence() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("main.py"), "def run(): pass").unwrap();
        std::fs::write(ws.path().join("doc.md"), "# Doc").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();

        AddRelation::execute(
            ws.path(),
            None,
            ElementId::for_symbol("main.py", "run"),
            DocumentId("doc.md".to_string()),
            Some(Confidence::High),
            false,
        )
        .unwrap();

        let res = AddRelation::execute(
            ws.path(),
            None,
            ElementId::for_symbol("main.py", "run"),
            DocumentId("doc.md".to_string()),
            Some(Confidence::Medium),
            false,
        )
        .unwrap();

        assert_eq!(res.result, InsertOutcome::Updated);
    }

    #[test]
    fn test_remove_relation_absent_pair_returns_zero_and_success() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("main.py"), "def run(): pass").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();

        let res = RemoveRelation::execute(
            ws.path(),
            None,
            ElementId::for_symbol("main.py", "run"),
            DocumentId("doc.md".to_string()),
            false,
        )
        .unwrap();

        assert_eq!(res.count, 0);
        assert!(!res.removed);
    }

    #[test]
    fn test_dry_run_does_not_write_file() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();
        std::fs::write(proj.path().join("main.py"), "def run(): pass").unwrap();
        std::fs::write(ws.path().join("doc.md"), "# Doc").unwrap();

        SetupProject::execute(ws.path(), proj.path(), false).unwrap();

        let res = AddRelation::execute(
            ws.path(),
            None,
            ElementId::for_symbol("main.py", "run"),
            DocumentId("doc.md".to_string()),
            None,
            true, // dry-run
        )
        .unwrap();

        assert_eq!(res.result, InsertOutcome::Added);
        assert!(!ws.path().join(".agentic-doc/relations.json").exists());
    }
}

//! Use case SetupProject : initialisation et liaison du workspace docs au projet code.

use crate::infrastructure::config::WorkspaceConfig;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SetupError {
    #[error("Le projet cible '{0}' est introuvable.")]
    ProjectNotFound(String),

    #[error("Le projet cible ne peut pas être le répertoire docs lui-même.")]
    CannotTargetSelf,

    #[error("Le projet docs est déjà configuré. Utilisez --force pour réinitialiser.")]
    AlreadyConfigured,

    #[error("Erreur d'E/S : {0}")]
    Io(#[from] std::io::Error),

    #[error("Erreur de stockage : {0}")]
    Storage(#[from] crate::infrastructure::filesystem::StorageError),
}

#[derive(Debug, Clone)]
pub struct SetupResult {
    pub project_docs: PathBuf,
    pub project: PathBuf,
    pub config: WorkspaceConfig,
    pub overwritten: bool,
}

pub struct SetupProject;

impl SetupProject {
    pub fn execute(
        workspace: &Path,
        project_path: &Path,
        force: bool,
    ) -> Result<SetupResult, SetupError> {
        let canonical_workspace = workspace.canonicalize().map_err(SetupError::Io)?;

        let project_target = if project_path.is_relative() {
            workspace.join(project_path)
        } else {
            project_path.to_path_buf()
        };

        let canonical_project = project_target
            .canonicalize()
            .map_err(|_| SetupError::ProjectNotFound(project_path.to_string_lossy().to_string()))?;

        if canonical_workspace == canonical_project {
            return Err(SetupError::CannotTargetSelf);
        }

        let config_path = canonical_workspace.join(".agentic-doc").join("config.json");
        let overwritten = config_path.exists();

        if overwritten && !force {
            return Err(SetupError::AlreadyConfigured);
        }

        let snapshots_dir = canonical_workspace.join(".agentic-doc").join("snapshots");
        fs::create_dir_all(&snapshots_dir)?;

        let relative_project_root = pathdiff_relative(&canonical_project, &canonical_workspace);

        let config = WorkspaceConfig::new(relative_project_root, ".".to_string());
        config.save(&canonical_workspace)?;

        Ok(SetupResult {
            project_docs: canonical_workspace,
            project: canonical_project,
            config,
            overwritten,
        })
    }
}

fn pathdiff_relative(target: &Path, base: &Path) -> String {
    // Calcul simple de chemin relatif entre deux répertoires canoniques
    let mut base_components: Vec<_> = base.components().collect();
    let mut target_components: Vec<_> = target.components().collect();

    while !base_components.is_empty()
        && !target_components.is_empty()
        && base_components[0] == target_components[0]
    {
        base_components.remove(0);
        target_components.remove(0);
    }

    let mut rel = PathBuf::new();
    for _ in 0..base_components.len() {
        rel.push("..");
    }
    for comp in target_components {
        rel.push(comp);
    }

    if rel.as_os_str().is_empty() {
        ".".to_string()
    } else {
        rel.to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_setup_project_success() {
        let ws = tempdir().unwrap();
        let proj = tempdir().unwrap();

        let result = SetupProject::execute(ws.path(), proj.path(), false).unwrap();
        assert!(!result.overwritten);

        assert!(ws.path().join(".agentic-doc").join("config.json").exists());
        assert!(ws.path().join(".agentic-doc").join("snapshots").exists());

        // Deuxième fois sans --force -> échec
        let err = SetupProject::execute(ws.path(), proj.path(), false).unwrap_err();
        assert!(matches!(err, SetupError::AlreadyConfigured));

        // Avec --force -> succès
        let result_force = SetupProject::execute(ws.path(), proj.path(), true).unwrap();
        assert!(result_force.overwritten);
    }

    #[test]
    fn test_setup_non_existent_project_fails() {
        let ws = tempdir().unwrap();
        let bad_proj = ws.path().join("non_existent_dir");

        let err = SetupProject::execute(ws.path(), &bad_proj, false).unwrap_err();
        assert!(matches!(err, SetupError::ProjectNotFound(_)));
    }
}

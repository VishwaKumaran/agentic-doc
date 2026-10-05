use std::path::Path;

pub fn should_ignore(path: &Path, root: &Path) -> bool {
    if path == root {
        return false;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with('.')
        || matches!(
            name,
            "__pycache__"
                | ".venv"
                | "venv"
                | "env"
                | "node_modules"
                | "target"
                | "build"
                | "dist"
                | ".agentic-doc"
                | ".git"
        )
}

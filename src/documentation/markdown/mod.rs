//! Adaptateur d'analyse pour la documentation Markdown.

use crate::documentation::analyzer::{DocumentationAnalyzer, DocumentationError};
use crate::domain::document::Document;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct MarkdownAnalyzer;

impl MarkdownAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MarkdownAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentationAnalyzer for MarkdownAnalyzer {
    fn analyze(&self, docs_root: &Path) -> Result<Vec<Document>, DocumentationError> {
        if !docs_root.exists() {
            return Err(DocumentationError::RootNotFound(
                docs_root.to_string_lossy().to_string(),
            ));
        }

        let canonical_root = docs_root.canonicalize().map_err(DocumentationError::Io)?;

        let mut documents = Vec::new();

        let walker = WalkDir::new(&canonical_root).into_iter();

        for entry in walker.filter_entry(|e| !should_ignore(e.path(), &canonical_root)) {
            let entry = entry.map_err(DocumentationError::WalkDir)?;
            let path = entry.path();

            if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str());
                if ext == Some("md") || ext == Some("markdown") {
                    let relative = path
                        .strip_prefix(&canonical_root)
                        .map_err(|_| {
                            DocumentationError::RootNotFound(path.to_string_lossy().to_string())
                        })?
                        .to_string_lossy()
                        .to_string();

                    let content = fs::read_to_string(path)?;
                    let title = extract_title(&content);

                    documents.push(Document::new(&relative, title, path.to_path_buf()));
                }
            }
        }

        documents.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(documents)
    }
}

fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return Some(title.trim().to_string());
        }
    }
    None
}

fn should_ignore(path: &Path, root: &Path) -> bool {
    if path == root {
        return false;
    }

    let name = match path.file_name().and_then(|s| s.to_str()) {
        Some(n) => n,
        None => return false,
    };

    if name.starts_with('.') {
        return true;
    }

    matches!(
        name,
        "node_modules" | "target" | "dist" | "build" | ".agentic-doc" | ".git"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_markdown_analyzer_extracts_documents_and_titles() {
        let temp = tempdir().unwrap();
        let doc1 = temp.path().join("index.md");
        let doc2 = temp.path().join("arch.markdown");

        fs::write(&doc1, "# Overview\nSome content").unwrap();
        fs::write(&doc2, "No title here").unwrap();

        let analyzer = MarkdownAnalyzer::new();
        let docs = analyzer.analyze(temp.path()).unwrap();

        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id.0, "arch.markdown");
        assert_eq!(docs[0].title, None);

        assert_eq!(docs[1].id.0, "index.md");
        assert_eq!(docs[1].title, Some("Overview".to_string()));
    }
}

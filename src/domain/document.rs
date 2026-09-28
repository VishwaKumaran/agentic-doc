//! Modèle de document de documentation.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DocumentId(pub String);

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub title: Option<String>,
    pub path: PathBuf,
}

impl Document {
    pub fn new(relative_path: &str, title: Option<String>, full_path: PathBuf) -> Self {
        Self {
            id: DocumentId(relative_path.to_string()),
            title,
            path: full_path,
        }
    }
}

//! Port d'extraction des symboles d'un langage source.

use crate::domain::source::SourceElement;

pub trait SourceParser: Send + Sync {
    fn language(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn parse(&self, relative_path: &str, content: &str) -> Vec<SourceElement>;
}

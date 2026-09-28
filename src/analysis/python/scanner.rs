//! Scanner textuel pour Python.
//!
//! # Limites du scanner
//! - Pas d'analyse sémantique complète ni d'AST.
//! - Détecte uniquement les `class` et `def` de niveau zéro (global) et les `def` indentés d'un niveau dans une `class`.
//! - Ne gère pas les décorateurs, ni les définitions imbriquées complexes.
//! - Le but est la stabilité du hachage et du résultat pour le MVP.

use crate::domain::hash::fnv1a_64;
use crate::domain::source::{ElementId, SourceElement, SourceElementKind};

pub fn scan_python_content(
    relative_path: &str,
    content: &str,
) -> (SourceElement, Vec<SourceElement>) {
    let file_hash = fnv1a_64(content.as_bytes());
    let lines: Vec<&str> = content.lines().collect();

    let file_element = SourceElement {
        id: ElementId::for_file(relative_path),
        kind: SourceElementKind::File,
        name: relative_path.to_string(),
        file: relative_path.to_string(),
        line_start: 1,
        line_end: lines.len().max(1),
        content_hash: file_hash,
    };

    let mut elements = Vec::new();
    let mut current_class: Option<String> = None;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if indent == 0 && trimmed.starts_with("class ") {
            if let Some(class_name) = extract_name(&trimmed[6..]) {
                current_class = Some(class_name.clone());
                let (end_line, text) = collect_block(&lines, i, 0);
                let elem = SourceElement {
                    id: ElementId::for_symbol(relative_path, &class_name),
                    kind: SourceElementKind::Class,
                    name: class_name,
                    file: relative_path.to_string(),
                    line_start: i + 1,
                    line_end: end_line,
                    content_hash: fnv1a_64(text.as_bytes()),
                };
                elements.push(elem);
            }
        } else if indent == 0 && trimmed.starts_with("def ") {
            current_class = None;
            if let Some(fn_name) = extract_name(&trimmed[4..]) {
                let (end_line, text) = collect_block(&lines, i, 0);
                let elem = SourceElement {
                    id: ElementId::for_symbol(relative_path, &fn_name),
                    kind: SourceElementKind::Function,
                    name: fn_name,
                    file: relative_path.to_string(),
                    line_start: i + 1,
                    line_end: end_line,
                    content_hash: fnv1a_64(text.as_bytes()),
                };
                elements.push(elem);
            }
        } else if indent > 0 && trimmed.starts_with("def ") {
            if let Some(ref class_name) = current_class {
                if let Some(method_name) = extract_name(&trimmed[4..]) {
                    let full_name = format!("{}.{}", class_name, method_name);
                    let (end_line, text) = collect_block(&lines, i, indent);
                    let elem = SourceElement {
                        id: ElementId::for_symbol(relative_path, &full_name),
                        kind: SourceElementKind::Method,
                        name: full_name,
                        file: relative_path.to_string(),
                        line_start: i + 1,
                        line_end: end_line,
                        content_hash: fnv1a_64(text.as_bytes()),
                    };
                    elements.push(elem);
                }
            }
        }

        i += 1;
    }

    (file_element, elements)
}

fn extract_name(s: &str) -> Option<String> {
    let name: String = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn collect_block(lines: &[&str], start_idx: usize, base_indent: usize) -> (usize, String) {
    let mut block_lines = vec![lines[start_idx]];
    let mut end_idx = start_idx + 1;

    while end_idx < lines.len() {
        let line = lines[end_idx];
        if line.trim().is_empty() {
            block_lines.push(line);
            end_idx += 1;
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        if indent <= base_indent {
            break;
        }
        block_lines.push(line);
        end_idx += 1;
    }

    (end_idx, block_lines.join("\n"))
}

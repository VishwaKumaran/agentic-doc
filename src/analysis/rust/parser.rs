use crate::analysis::parser::SourceParser;
use crate::domain::hash::fnv1a_64;
use crate::domain::source::{ElementId, SourceElement, SourceElementKind};
use tree_sitter::{Node, Parser};

pub struct RustParser;
impl SourceParser for RustParser {
    fn language(&self) -> &'static str {
        "Rust"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }
    fn parse(&self, path: &str, content: &str) -> Vec<SourceElement> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("Rust grammar is valid");
        let tree = parser
            .parse(content, None)
            .expect("tree-sitter returns a tree");
        let mut out = Vec::new();
        let mut impl_counts = std::collections::BTreeMap::new();
        visit(
            tree.root_node(),
            path,
            content,
            "",
            None,
            &mut impl_counts,
            &mut out,
        );
        out
    }
}
fn visit(
    node: Node<'_>,
    path: &str,
    source: &str,
    prefix: &str,
    impl_type: Option<&str>,
    counts: &mut std::collections::BTreeMap<String, usize>,
    out: &mut Vec<SourceElement>,
) {
    match node.kind() {
        "struct_item" => named(node, path, source, prefix, SourceElementKind::Struct, out),
        "enum_item" => named(node, path, source, prefix, SourceElementKind::Enum, out),
        "trait_item" => named(node, path, source, prefix, SourceElementKind::Trait, out),
        "function_item" => {
            let name = field(node, "name", source);
            let qualified = match impl_type {
                Some(ty) => join(ty, name),
                None => join(prefix, name),
            };
            push(
                node,
                path,
                source,
                qualified,
                if impl_type.is_some() {
                    SourceElementKind::Method
                } else {
                    SourceElementKind::Function
                },
                out,
            );
        }
        "mod_item" => {
            let name = field(node, "name", source);
            let qualified = join(prefix, name);
            push(
                node,
                path,
                source,
                qualified.clone(),
                SourceElementKind::Module,
                out,
            );
            if let Some(body) = node.child_by_field_name("body") {
                for child in body.named_children(&mut body.walk()) {
                    visit(child, path, source, &qualified, None, counts, out);
                }
            }
        }
        "impl_item" => {
            let type_name = field(node, "type", source);
            let base = join(prefix, type_name);
            let trait_name = node
                .child_by_field_name("trait")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok());
            let key = format!("{base}::impl");
            let count = counts.entry(key.clone()).or_insert(0);
            *count += 1;
            let impl_name = match trait_name {
                Some(trait_name) => format!("{key}::{trait_name}"),
                None if *count == 1 => key,
                None => format!("{key}#{}", *count),
            };
            push(node, path, source, impl_name, SourceElementKind::Impl, out);
            if let Some(body) = node.child_by_field_name("body") {
                for child in body.named_children(&mut body.walk()) {
                    visit(child, path, source, prefix, Some(&base), counts, out);
                }
            }
        }
        _ => {
            for child in node.named_children(&mut node.walk()) {
                visit(child, path, source, prefix, impl_type, counts, out);
            }
        }
    }
}
fn named(
    node: Node<'_>,
    path: &str,
    source: &str,
    prefix: &str,
    kind: SourceElementKind,
    out: &mut Vec<SourceElement>,
) {
    push(
        node,
        path,
        source,
        join(prefix, field(node, "name", source)),
        kind,
        out,
    );
}
fn field<'a>(node: Node<'_>, name: &str, source: &'a str) -> &'a str {
    node.child_by_field_name(name)
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("")
}
fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}::{name}")
    }
}
fn push(
    node: Node<'_>,
    path: &str,
    source: &str,
    name: String,
    kind: SourceElementKind,
    out: &mut Vec<SourceElement>,
) {
    out.push(SourceElement {
        id: ElementId::for_symbol(path, &name),
        kind,
        name,
        file: path.to_string(),
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        content_hash: fnv1a_64(source[node.byte_range()].as_bytes()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rust_items_and_ignores_local_items() {
        let source = "struct User;\nenum State { Ready }\ntrait Run {}\nimpl User { fn new() {} }\nimpl User { fn reset() {} }\nimpl Run for User { fn run() {} }\nmod api { fn call() {} }\nfn outer() { fn local() {} }\n";
        let elements = RustParser.parse("src/lib.rs", source);
        let ids: Vec<_> = elements.iter().map(|e| e.id.0.as_str()).collect();
        assert!(ids.contains(&"src/lib.rs::User"));
        assert!(ids.contains(&"src/lib.rs::User::impl"));
        assert!(ids.contains(&"src/lib.rs::User::impl#2"));
        assert!(ids.contains(&"src/lib.rs::User::impl::Run"));
        assert!(ids.contains(&"src/lib.rs::User::new"));
        assert!(ids.contains(&"src/lib.rs::api::call"));
        assert!(ids.contains(&"src/lib.rs::outer"));
        assert!(!ids.iter().any(|id| id.ends_with("::local")));
    }
}

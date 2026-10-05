use crate::analysis::parser::SourceParser;
use crate::domain::hash::fnv1a_64;
use crate::domain::source::{ElementId, SourceElement, SourceElementKind};
use tree_sitter::{Node, Parser};

pub struct PythonParser;
impl SourceParser for PythonParser {
    fn language(&self) -> &'static str {
        "Python"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["py"]
    }
    fn parse(&self, path: &str, content: &str) -> Vec<SourceElement> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .expect("Python grammar is valid");
        let tree = parser
            .parse(content, None)
            .expect("tree-sitter returns a tree");
        let mut out = Vec::new();
        visit(tree.root_node(), path, content, "", false, &mut out);
        out
    }
}

fn visit(
    node: Node<'_>,
    path: &str,
    source: &str,
    prefix: &str,
    in_class: bool,
    out: &mut Vec<SourceElement>,
) {
    match node.kind() {
        "function_definition" => {
            let name = field_text(node, "name", source);
            let qualified = join(prefix, name);
            push(
                node,
                path,
                source,
                qualified,
                if in_class {
                    SourceElementKind::Method
                } else {
                    SourceElementKind::Function
                },
                out,
            );
        }
        "class_definition" => {
            let name = field_text(node, "name", source);
            let qualified = join(prefix, name);
            push(
                node,
                path,
                source,
                qualified.clone(),
                SourceElementKind::Class,
                out,
            );
            if let Some(body) = node.child_by_field_name("body") {
                for child in body.named_children(&mut body.walk()) {
                    visit(child, path, source, &qualified, true, out);
                }
            }
        }
        "decorated_definition" => {
            if let Some(definition) = node.child_by_field_name("definition") {
                match definition.kind() {
                    "function_definition" => {
                        let name = field_text(definition, "name", source);
                        push(
                            node,
                            path,
                            source,
                            join(prefix, name),
                            if in_class {
                                SourceElementKind::Method
                            } else {
                                SourceElementKind::Function
                            },
                            out,
                        );
                    }
                    "class_definition" => {
                        let name = field_text(definition, "name", source);
                        let qualified = join(prefix, name);
                        push(
                            node,
                            path,
                            source,
                            qualified.clone(),
                            SourceElementKind::Class,
                            out,
                        );
                        if let Some(body) = definition.child_by_field_name("body") {
                            for child in body.named_children(&mut body.walk()) {
                                visit(child, path, source, &qualified, true, out);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {
            for child in node.named_children(&mut node.walk()) {
                visit(child, path, source, prefix, in_class, out);
            }
        }
    }
}
fn field_text<'a>(node: Node<'_>, field: &str, source: &'a str) -> &'a str {
    node.child_by_field_name(field)
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
    let start = node.start_position().row + 1;
    let end = node.end_position().row + 1;
    let text = &source[node.byte_range()];
    out.push(SourceElement {
        id: ElementId::for_symbol(path, &name),
        kind,
        name,
        file: path.to_string(),
        line_start: start,
        line_end: end,
        content_hash: fnv1a_64(text.as_bytes()),
    });
}

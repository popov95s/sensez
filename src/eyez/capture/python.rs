//! `eyez`-only: capture Python docstrings + comments during the walk into
//! [`Walked::docs`](crate::spine::parser::Walked). This module reads node text only; it
//! never writes to the structural `tokens`/`lexemes`/`spans` the duplication
//! pillar consumes, so it cannot affect clone detection.

use crate::eyez::{DocKind, RawDoc};
use crate::spine::ir::Walked;
use tree_sitter::Node;

/// True when a bare `string`/`concatenated_string` (whose parent is an
/// `expression_statement`) sits in docstring position: the first statement of its
/// enclosing module/class/function body. Leading comments are allowed before it.
pub fn is_docstring(string_node: Node) -> bool {
    let Some(stmt) = string_node.parent() else {
        return false;
    };
    let Some(body) = stmt.parent() else {
        return false;
    };
    if !matches!(body.kind(), "module" | "block") {
        return false;
    }
    let mut cursor = body.walk();
    let first = body
        .named_children(&mut cursor)
        .find(|n| n.kind() != "comment");
    first.is_some_and(|first| first.id() == stmt.id())
}

/// Record a docstring for `scope_path` (empty ⇒ module-level). No-op if blank.
pub fn push_docstring(out: &mut Walked, module: &str, scope_path: &[&str], node: Node, src: &[u8]) {
    let text = string_text(node, src);
    if !text.is_empty() {
        let line = node.start_position().row + 1;
        out.docs.push(RawDoc::new(
            module,
            scope_path,
            DocKind::Docstring,
            text,
            line,
        ));
    }
}

/// Record a comment (`# …`) attributed to the enclosing `scope_path`.
pub fn push_comment(out: &mut Walked, module: &str, scope_path: &[&str], node: Node, src: &[u8]) {
    if let Ok(raw) = node.utf8_text(src) {
        let text = raw.trim_start_matches('#').trim();
        if !text.is_empty() {
            out.docs.push(RawDoc::new(
                module,
                scope_path,
                DocKind::Comment,
                text.to_string(),
                node.start_position().row + 1,
            ));
        }
    }
}

/// Literal text inside a `string`/`concatenated_string`, quotes/prefixes removed.
fn string_text(node: Node, src: &[u8]) -> String {
    let mut out = String::new();
    collect_content(node, src, &mut out);
    if out.is_empty() {
        if let Ok(raw) = node.utf8_text(src) {
            out = raw.trim_matches(|c| c == '"' || c == '\'').to_string();
        }
    }
    out.trim().to_string()
}

fn collect_content(node: Node, src: &[u8], out: &mut String) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "string_content" => {
                if let Ok(t) = child.utf8_text(src) {
                    out.push_str(t);
                }
            }
            "string" => collect_content(child, src, out),
            _ => {}
        }
    }
}

//! `eyez`-only: capture Rust comments into [`Walked::docs`](crate::spine::parser::Walked).

use crate::eyez::{DocKind, RawDoc};
use crate::spine::ir::Walked;
use tree_sitter::Node;

/// Record a Rust comment for `scope_path`; doc comments index as docstrings.
pub fn push_comment(out: &mut Walked, module: &str, scope_path: &[&str], node: Node, src: &[u8]) {
    let raw = super::lossy_text(node, src);
    {
        let is_doc = raw.starts_with("///") || raw.starts_with("//!") || raw.starts_with("/**");
        let text = clean(&raw);
        if !text.is_empty() {
            let kind = if is_doc {
                DocKind::Docstring
            } else {
                DocKind::Comment
            };
            let line = node.start_position().row + 1;
            out.docs
                .push(RawDoc::new(module, scope_path, kind, text, line));
        }
    }
}

/// Strip comment delimiters (`///`, `//!`, `//`, `/*…*/`) and per-line markers.
fn clean(raw: &str) -> String {
    let body = raw
        .trim()
        .trim_start_matches("/**")
        .trim_start_matches("/*!")
        .trim_start_matches("/*")
        .trim_end_matches("*/");
    body.lines()
        .map(|l| {
            l.trim()
                .trim_start_matches("///")
                .trim_start_matches("//!")
                .trim_start_matches("//")
                .trim_start_matches('*')
                .trim()
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

//! `eyez`-only: capture Rust comments (incl. `///` and `//!` doc comments)
//! during the walk into [`Walked::docs`](crate::spine::parser::Walked). Reads node
//! text only — never writes to the structural token stream, so duplication is
//! unaffected.

use crate::eyez::{DocKind, RawDoc};
use crate::spine::ir::Walked;
use tree_sitter::Node;

/// Record a `//`, `///`, `//!`, or `/* … */` comment for `scope_path`. Doc
/// comments index as docstrings (they document the following item); plain
/// comments index as comments.
pub fn push_comment(out: &mut Walked, module: &str, scope_path: &[&str], node: Node, src: &[u8]) {
    if let Ok(raw) = node.utf8_text(src) {
        let is_doc = raw.starts_with("///") || raw.starts_with("//!") || raw.starts_with("/**");
        let text = clean(raw);
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

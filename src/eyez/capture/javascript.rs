//! `eyez`-only: capture JS/TS comments + JSDoc during the walk into
//! [`Walked::docs`](crate::spine::parser::Walked). Reads node text only — never writes
//! to the structural token stream, so duplication is unaffected. JS bare strings
//! are directive prologues (`"use strict"`), not docs, so only comments are
//! indexed here.

use crate::eyez::{DocKind, RawDoc};
use crate::spine::ir::Walked;
use tree_sitter::Node;

/// Record a `//`, `/* … */`, or JSDoc (`/** … */`) comment for `scope_path`.
pub fn push_comment(out: &mut Walked, module: &str, scope_path: &[&str], node: Node, src: &[u8]) {
    if let Ok(raw) = node.utf8_text(src) {
        let text = clean(raw);
        if !text.is_empty() {
            let line = node.start_position().row + 1;
            out.docs.push(RawDoc::new(
                module,
                scope_path,
                DocKind::Comment,
                text,
                line,
            ));
        }
    }
}

/// Strip comment delimiters (`//`, `/*`, `*/`) and per-line leading `*` markers.
fn clean(raw: &str) -> String {
    let body = raw
        .trim()
        .trim_start_matches("/**")
        .trim_start_matches("/*")
        .trim_start_matches("//")
        .trim_end_matches("*/");
    body.lines()
        .map(|l| l.trim().trim_start_matches('*').trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

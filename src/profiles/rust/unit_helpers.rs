//! Small Rust AST helpers for unit extraction.

use crate::profiles::walk::{self, MutationTarget};
use tree_sitter::Node;

pub(super) fn target_root(node: Node, src: &[u8]) -> Option<MutationTarget> {
    match node.kind() {
        "identifier" => walk::node_text(node, src).map(|id| MutationTarget::Direct(id.to_string())),
        "self" => Some(MutationTarget::Direct("self".to_string())),
        "index_expression" => target_root(node.named_child(0)?, src),
        "field_expression" => {
            target_root(node.child_by_field_name("value")?, src).map(MutationTarget::via_attribute)
        }
        _ => None,
    }
}

pub(super) fn pattern_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => walk::node_text(node, src).map(str::to_string),
        "mut_pattern" | "ref_pattern" | "reference_pattern" => {
            node.named_child(0).and_then(|n| pattern_name(n, src))
        }
        "tuple_pattern" | "slice_pattern" | "struct_pattern" | "tuple_struct_pattern" => None,
        _ => None,
    }
}

pub(super) fn type_text(node: Node, src: &[u8]) -> Option<String> {
    let text = walk::node_text(node, src)?;
    Some(text.trim_start_matches("->").trim().to_string())
}

pub(super) fn tuple_type_arity(ty: &str) -> usize {
    let text = ty.trim();
    if !(text.starts_with('(') && text.ends_with(')')) {
        return 0;
    }
    let body = &text[1..text.len().saturating_sub(1)];
    if body.trim().is_empty() {
        return 0;
    }
    body.chars().fold(ArityScan::new(), ArityScan::scan).count
}

struct ArityScan {
    depth: usize,
    count: usize,
}

impl ArityScan {
    fn new() -> Self {
        Self { depth: 0, count: 1 }
    }

    fn scan(mut self, ch: char) -> Self {
        match ch {
            '<' | '(' | '[' => self.depth += 1,
            '>' | ')' | ']' => self.depth = self.depth.saturating_sub(1),
            ',' if self.depth == 0 => self.count += 1,
            _ => {}
        }
        self
    }
}

pub(super) fn is_string(kind: &str) -> bool {
    matches!(kind, "string_literal" | "raw_string_literal")
}

pub(super) fn unquote(text: &str) -> String {
    text.trim_matches('"').trim_matches('\'').to_string()
}

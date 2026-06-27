//! Small Rust AST helpers for unit extraction.

use crate::profiles::walk;
use crate::spine::ir::CallFact;
use tree_sitter::Node;

pub(super) fn call_fact(node: Node, src: &[u8]) -> Option<CallFact> {
    let func = node.child_by_field_name("function")?;
    let line = node.start_position().row + 1;
    match func.kind() {
        "identifier" => Some(CallFact::named(walk::node_text(func, src)?, line)),
        "field_expression" => {
            let base = func
                .child_by_field_name("value")
                .and_then(|n| root_ident(n, src))?;
            let method = func
                .child_by_field_name("field")
                .and_then(|n| walk::node_text(n, src))?;
            Some(CallFact::member(&base, method, line))
        }
        _ => None,
    }
}

fn root_ident(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => walk::node_text(node, src).map(str::to_string),
        "self" => Some("self".to_string()),
        "field_expression" => node
            .child_by_field_name("value")
            .and_then(|n| root_ident(n, src)),
        _ => None,
    }
}

pub(super) fn target_root(node: Node, src: &[u8]) -> Option<(String, bool)> {
    match node.kind() {
        "identifier" => walk::node_text(node, src).map(|id| (id.to_string(), false)),
        "self" => Some(("self".to_string(), false)),
        "index_expression" => target_root(node.named_child(0)?, src),
        "field_expression" => {
            target_root(node.child_by_field_name("value")?, src).map(|(root, _)| (root, true))
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

pub(super) fn chain_len(node: Node) -> usize {
    match node.child_by_field_name("value") {
        Some(value) if value.kind() == "field_expression" => chain_len(value) + 1,
        _ => 1,
    }
}

pub(super) fn is_loop(kind: &str) -> bool {
    matches!(
        kind,
        "for_expression" | "while_expression" | "loop_expression"
    )
}

pub(super) fn is_nesting(kind: &str) -> bool {
    matches!(
        kind,
        "if_expression"
            | "match_expression"
            | "for_expression"
            | "while_expression"
            | "loop_expression"
    )
}

pub(super) fn is_branch(kind: &str, node: Node, src: &[u8]) -> bool {
    match kind {
        "if_expression" | "match_expression" | "for_expression" | "while_expression" => true,
        "binary_expression" => node
            .child_by_field_name("operator")
            .and_then(|n| walk::node_text(n, src))
            .is_some_and(|op| matches!(op, "&&" | "||")),
        _ => false,
    }
}

pub(super) fn cognitive_weight(kind: &str, depth: usize) -> Option<usize> {
    match kind {
        "if_expression" | "match_expression" | "for_expression" | "while_expression" => {
            Some(1 + depth)
        }
        _ => None,
    }
}

pub(super) fn is_string(kind: &str) -> bool {
    matches!(kind, "string_literal" | "raw_string_literal")
}

pub(super) fn unquote(text: &str) -> String {
    text.trim_matches('"').trim_matches('\'').to_string()
}

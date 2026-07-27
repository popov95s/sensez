//! Small Rust AST helpers for unit extraction.

use crate::profiles::walk;
use crate::spine::ir::CallFact;
use tree_sitter::Node;

pub(super) fn call_fact(node: Node, src: &[u8]) -> Option<CallFact> {
    let mut func = node.child_by_field_name("function")?;
    if func.kind() == "generic_function" {
        func = func
            .child_by_field_name("function")
            .or_else(|| func.named_child(0))?;
    }
    let line = node.start_position().row + 1;
    let mut call = match func.kind() {
        "identifier" => Some(CallFact::named(walk::node_text(func, src)?, line)),
        "field_expression" => {
            let base = func
                .child_by_field_name("value")
                .and_then(|n| receiver_path(n, src))?;
            let method = func
                .child_by_field_name("field")
                .and_then(|n| walk::node_text(n, src))?;
            Some(CallFact::member(&base, method, line))
        }
        _ => None,
    }?;
    call.region = control_region(node);
    Some(call)
}

fn receiver_path(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => walk::node_text(node, src).map(str::to_string),
        "self" => Some("self".to_string()),
        "field_expression" => {
            let value = receiver_path(node.child_by_field_name("value")?, src)?;
            let field = walk::node_text(node.child_by_field_name("field")?, src)?;
            Some(format!("{value}.{field}"))
        }
        "call_expression" => {
            let mut function = node.child_by_field_name("function")?;
            if function.kind() == "generic_function" {
                function = function.named_child(0)?;
            }
            let base = receiver_path(function.child_by_field_name("value")?, src)?;
            let method = walk::node_text(function.child_by_field_name("field")?, src)?;
            if is_lazy_adapter(method) {
                Some(base)
            } else {
                Some(format!("{base}.{method}"))
            }
        }
        _ => None,
    }
}

fn is_lazy_adapter(method: &str) -> bool {
    matches!(
        method,
        "enumerate"
            | "filter"
            | "filter_map"
            | "flat_map"
            | "into_iter"
            | "iter"
            | "iter_mut"
            | "map"
            | "skip"
            | "take"
            | "zip"
    )
}

fn control_region(node: Node<'_>) -> usize {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if matches!(parent.kind(), "if_expression" | "match_arm") {
            return current.start_position().row + 1;
        }
        if parent.kind() == "function_item" {
            break;
        }
        current = parent;
    }
    0
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

//! Rust call facts: callee, receiver path and conditional region.

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
    call.offset = node.start_byte();
    call.awaited = node
        .parent()
        .is_some_and(|parent| parent.kind() == "await_expression");
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

/// Method name and receiver of a `receiver.method(..)` call.
pub(super) fn method_call<'a>(call: Node<'a>, src: &'a [u8]) -> Option<(&'a str, Node<'a>)> {
    let function = call
        .child_by_field_name("function")
        .filter(|f| f.kind() == "field_expression")?;
    let method = function.child_by_field_name("field")?.utf8_text(src).ok()?;
    Some((method, function.child_by_field_name("value")?))
}

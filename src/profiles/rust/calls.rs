//! Rust call facts: callee, receiver path and conditional region.

use crate::profiles::node_names::NodeNames;
use crate::profiles::walk;
use crate::spine::ir::CallFact;
use tree_sitter::Node;

pub(super) fn call_fact(node: Node, src: &[u8]) -> Option<CallFact> {
    let mut func = node.field("function")?;
    if func.kind_name() == "generic_function" {
        func = func.field("function").or_else(|| func.named_child(0))?;
    }
    let line = node.start_position().row + 1;
    let mut call = match func.kind_name() {
        "identifier" => Some(CallFact::named(walk::node_text(func, src)?, line)),
        "field_expression" => {
            let base = func.field("value").and_then(|n| receiver_path(n, src))?;
            let method = func.field("field").and_then(|n| walk::node_text(n, src))?;
            Some(CallFact::member(&base, method, line))
        }
        _ => None,
    }?;
    call.region = control_region(node);
    call.offset = node.start_byte();
    call.awaited = node
        .parent()
        .is_some_and(|parent| parent.kind_name() == "await_expression");
    Some(call)
}

fn receiver_path(node: Node, src: &[u8]) -> Option<String> {
    match node.kind_name() {
        "identifier" => walk::node_text(node, src).map(str::to_string),
        "self" => Some("self".to_string()),
        "field_expression" => {
            let value = receiver_path(node.field("value")?, src)?;
            let field = walk::node_text(node.field("field")?, src)?;
            Some(format!("{value}.{field}"))
        }
        "call_expression" => {
            let mut function = node.field("function")?;
            if function.kind_name() == "generic_function" {
                function = function.named_child(0)?;
            }
            let base = receiver_path(function.field("value")?, src)?;
            let method = walk::node_text(function.field("field")?, src)?;
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
        if matches!(parent.kind_name(), "if_expression" | "match_arm") {
            return current.start_position().row + 1;
        }
        if parent.kind_name() == "function_item" {
            break;
        }
        current = parent;
    }
    0
}

/// Method name and receiver of a `receiver.method(..)` call.
pub(super) fn method_call<'a>(call: Node<'a>, src: &'a [u8]) -> Option<(&'a str, Node<'a>)> {
    let function = call
        .field("function")
        .filter(|f| f.kind_name() == "field_expression")?;
    let method = function.field("field")?.utf8_text(src).ok()?;
    Some((method, function.field("value")?))
}

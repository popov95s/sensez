//! JS/TS AST facts for performance-oriented smells.

use crate::profiles::walk;
use crate::spine::ir::{CallFact, PerfLine, PerformanceFacts};
use tree_sitter::Node;

const ITER_METHODS: [&str; 9] = [
    "some",
    "every",
    "filter",
    "map",
    "reduce",
    "reduceRight",
    "find",
    "findIndex",
    "forEach",
];
const EXPENSIVE_LOOP_METHODS: &[&str] = &[
    "all", "execute", "fetch", "fetchOne", "find", "findOne", "load", "query", "request", "save",
];
const MUTATING_METHODS: &[&str] = &[
    "copyWithin",
    "fill",
    "pop",
    "push",
    "reverse",
    "shift",
    "sort",
    "splice",
    "unshift",
];
const STRONG_IO_METHODS: &[&str] = &["execute", "query", "request"];
const EXTERNAL_RECEIVER_NAMES: &[&str] = &[
    "api",
    "client",
    "connection",
    "db",
    "repo",
    "repository",
    "session",
];
const EXTERNAL_TYPE_PARTS: &[&str] = &[
    "client",
    "connection",
    "database",
    "repository",
    "session",
    "transport",
];
pub fn scan(facts: &mut PerformanceFacts, node: Node, src: &[u8], loop_depth: usize) {
    let kind = node.kind();
    if is_loop(kind) {
        let loop_line = line(node, src);
        facts.loops.push(loop_line.clone());
        if loop_depth > 0 {
            facts.nested_loops.push(loop_line);
        }
    }
    if kind != "call_expression" {
        return;
    }
    let Some(call) = call_fact(node, src) else {
        return;
    };
    facts.calls.push(call.clone());
    if call.member && ITER_METHODS.contains(&call.method.as_str()) {
        facts.iteration_calls.push(call.clone());
    }
    if loop_depth == 0 {
        return;
    }
    if call.member && call.method == "sort" {
        facts.sorts_in_loops.push(PerfLine {
            line: call.line,
            subject: call.base.clone(),
        });
    } else {
        facts.loop_calls.push(call);
    }
}

pub fn is_loop(kind: &str) -> bool {
    matches!(
        kind,
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
    )
}

fn call_fact(node: Node, src: &[u8]) -> Option<CallFact> {
    let func = node.child_by_field_name("function")?;
    let line = node.start_position().row + 1;
    let mut call = match func.kind() {
        "identifier" => Some(CallFact::named(walk::node_text(func, src)?, line)),
        "member_expression" => {
            let base = func
                .child_by_field_name("object")
                .filter(stable_receiver)
                .and_then(|n| walk::node_text(n, src))?;
            let method = func
                .child_by_field_name("property")
                .and_then(|n| walk::node_text(n, src))?;
            Some(CallFact::member(base, method, line))
        }
        _ => None,
    }?;
    call.region = control_region(node);
    Some(call)
}

fn stable_receiver(node: &Node<'_>) -> bool {
    matches!(node.kind(), "identifier" | "this")
        || (node.kind() == "member_expression"
            && node
                .child_by_field_name("object")
                .is_some_and(|base| stable_receiver(&base)))
}

fn control_region(node: Node<'_>) -> usize {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if matches!(
            parent.kind(),
            "if_statement" | "switch_case" | "ternary_expression"
        ) {
            let alternative = parent
                .child_by_field_name("alternative")
                .is_some_and(|branch| branch.id() == current.id());
            return if alternative {
                current.start_position().row + 1
            } else {
                parent.start_position().row + 1
            };
        }
        if super::units::is_function(parent.kind()) {
            break;
        }
        current = parent;
    }
    0
}

fn line(node: Node, src: &[u8]) -> PerfLine {
    walk::perf_line(node, src, &["right", "value"])
}

pub(crate) fn is_mutating_call(method: &str) -> bool {
    MUTATING_METHODS.contains(&method)
}

pub(crate) fn receiver_root(receiver: &str) -> &str {
    receiver.split('.').next().unwrap_or(receiver)
}

pub(crate) fn is_bounded_loop(subject: &str) -> bool {
    let compact: String = subject.chars().filter(|ch| !ch.is_whitespace()).collect();
    !compact.is_empty()
        && ((compact.starts_with('[') && compact.ends_with(']'))
            || (compact
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch == '_')))
        && compact.split(',').count() <= 12
}

pub(crate) fn is_external_loop_call(
    method: &str,
    receiver: &str,
    receiver_type: Option<&str>,
    _loops: &[PerfLine],
) -> bool {
    if !EXPENSIVE_LOOP_METHODS.contains(&method) {
        return false;
    }
    if STRONG_IO_METHODS.contains(&method) {
        return true;
    }
    receiver_type.is_some_and(|type_name| {
        let lower = type_name.to_ascii_lowercase();
        EXTERNAL_TYPE_PARTS.iter().any(|part| lower.contains(part))
    }) || EXTERNAL_RECEIVER_NAMES
        .iter()
        .any(|word| receiver.to_ascii_lowercase().contains(word))
}

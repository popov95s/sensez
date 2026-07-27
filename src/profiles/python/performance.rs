//! Python AST facts for performance-oriented smells.

use crate::profiles::walk;
use crate::spine::ir::{CallFact, PerfLine, PerformanceFacts};
use tree_sitter::Node;

const ITER_FUNCTIONS: [&str; 8] = ["any", "all", "sum", "min", "max", "sorted", "list", "set"];
const ITER_METHODS: [&str; 3] = ["count", "index", "copy"];
const EXPENSIVE_LOOP_METHODS: &[&str] = &[
    "all", "execute", "fetch", "fetchone", "find", "load", "query", "request", "save", "select",
];
const MUTATING_METHODS: &[&str] = &[
    "add", "append", "clear", "discard", "extend", "insert", "pop", "remove", "reverse", "sort",
    "update",
];
const STRONG_IO_METHODS: &[&str] = &["execute", "query", "request", "select"];
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
    if kind != "call" {
        return;
    }
    let Some(call) = call_fact(node, src) else {
        return;
    };
    facts.calls.push(call.clone());
    if let Some(iter) = iteration_call(node, src, &call) {
        facts.iteration_calls.push(iter);
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
    matches!(kind, "for_statement" | "while_statement")
}

fn call_fact(node: Node, src: &[u8]) -> Option<CallFact> {
    let func = node.child_by_field_name("function")?;
    let line = node.start_position().row + 1;
    let mut call = match func.kind() {
        "identifier" => Some(CallFact::named(walk::node_text(func, src)?, line)),
        "attribute" => {
            let base = func
                .child_by_field_name("object")
                .filter(|n| stable_receiver(n))
                .and_then(|n| walk::node_text(n, src))?;
            let method = func
                .child_by_field_name("attribute")
                .and_then(|n| walk::node_text(n, src))?;
            Some(CallFact::member(base, method, line))
        }
        _ => None,
    }?;
    call.region = control_region(node);
    Some(call)
}

fn iteration_call(node: Node, src: &[u8], call: &CallFact) -> Option<CallFact> {
    if call.member && ITER_METHODS.contains(&call.method.as_str()) {
        return Some(call.clone());
    }
    if !call.member && ITER_FUNCTIONS.contains(&call.method.as_str()) {
        let base = first_stable_arg(node, src)?;
        return Some(CallFact {
            base: base.to_string(),
            ..call.clone()
        });
    }
    None
}

fn first_stable_arg<'a>(call: Node, src: &'a [u8]) -> Option<&'a str> {
    let args = call.child_by_field_name("arguments")?;
    let mut cursor = args.walk();
    let result = args
        .named_children(&mut cursor)
        .find(|n| stable_receiver(n))
        .and_then(|n| walk::node_text(n, src));
    result
}

fn stable_receiver(node: &Node<'_>) -> bool {
    matches!(node.kind(), "identifier" | "attribute")
        && node.named_child_count() <= 2
        && (node.kind() == "identifier"
            || node
                .child_by_field_name("object")
                .is_some_and(|base| stable_receiver(&base)))
}

fn control_region(node: Node<'_>) -> usize {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if matches!(
            parent.kind(),
            "if_statement" | "elif_clause" | "conditional_expression"
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
        if parent.kind() == "function_definition" {
            break;
        }
        current = parent;
    }
    0
}

fn line(node: Node, src: &[u8]) -> PerfLine {
    walk::perf_line(node, src, &["right"])
}

pub(crate) fn is_mutating_call(method: &str) -> bool {
    MUTATING_METHODS.contains(&method)
}

pub(crate) fn receiver_root(receiver: &str) -> &str {
    receiver.split('.').next().unwrap_or(receiver)
}

pub(crate) fn is_bounded_loop(subject: &str) -> bool {
    let compact: String = subject.chars().filter(|ch| !ch.is_whitespace()).collect();
    is_constant_name(&compact)
        || is_small_literal_range(&compact)
        || is_literal_collection(&compact)
}

pub(crate) fn is_external_loop_call(
    method: &str,
    receiver: &str,
    receiver_type: Option<&str>,
    loops: &[PerfLine],
) -> bool {
    if !EXPENSIVE_LOOP_METHODS.contains(&method) {
        return false;
    }
    if STRONG_IO_METHODS.contains(&method) {
        return true;
    }
    if method == "load"
        && loops.iter().any(|loop_fact| {
            let subject = loop_fact.subject.to_ascii_lowercase();
            subject.contains("entry_points") || subject.contains("metadata")
        })
    {
        return true;
    }
    has_external_receiver(receiver, receiver_type)
}

fn is_constant_name(subject: &str) -> bool {
    !subject.is_empty()
        && subject
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch == '_')
}

fn is_small_literal_range(subject: &str) -> bool {
    let body = subject
        .strip_prefix("range(")
        .and_then(|text| text.strip_suffix(')'));
    body.is_some_and(|args| {
        args.split(',').all(|part| part.parse::<i32>().is_ok())
            && args
                .split(',')
                .filter_map(|part| part.parse::<i32>().ok())
                .all(|value| value.unsigned_abs() <= 100)
    })
}

fn is_literal_collection(subject: &str) -> bool {
    let wrapped = (subject.starts_with('[') && subject.ends_with(']'))
        || (subject.starts_with('(') && subject.ends_with(')'));
    wrapped && subject.split(',').count() <= 12
}

fn has_external_receiver(receiver: &str, receiver_type: Option<&str>) -> bool {
    receiver_type.is_some_and(|type_name| {
        let lower = type_name.to_ascii_lowercase();
        EXTERNAL_TYPE_PARTS.iter().any(|part| lower.contains(part))
    }) || EXTERNAL_RECEIVER_NAMES
        .iter()
        .any(|word| receiver.to_ascii_lowercase().contains(word))
}

//! JS/TS AST facts for performance-oriented smells.

use crate::profiles::lineage::Lineage;
use crate::profiles::walk;
use crate::spine::ir::{CallFact, IterationPass, PerfLine, PerformanceFacts};
use tree_sitter::Node;

const FULL_PASS_METHODS: &[&str] = &[
    "filter",
    "map",
    "reduce",
    "reduceRight",
    "forEach",
    "flatMap",
];
const ASSERTION_CALLEES: &[&str] = &["expect", "assert"];
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
pub fn scan(facts: &mut PerformanceFacts, lineage: Lineage, src: &[u8], frame: Option<usize>) {
    let node = lineage.node();
    let kind = node.kind();
    match kind {
        "return_statement" | "throw_statement" => facts.exits.push(node.end_byte()),
        "variable_declarator" => {
            record_bindings(facts, node, src);
            let names = super::loops::declared_names(node, src);
            facts.note_declared(frame, &names);
            if node
                .child_by_field_name("value")
                .is_some_and(|value| walk::fed_by_call(facts, frame, value, src, "call_expression"))
            {
                facts.note_call_results(frame, &names);
            }
        }
        _ => {}
    }
    if kind != "call_expression" {
        return;
    }
    let Some(mut call) = call_fact(lineage, src) else {
        return;
    };
    facts.calls.push(call.clone());
    if let Some(iteration) = iteration_call(lineage, src, &call) {
        facts.iteration_calls.push(iteration);
    }
    if frame.is_none() {
        return;
    }
    call.frame = frame;
    if call.member && call.method == "sort" {
        if facts.in_loop_statement(frame) {
            facts.sorts_in_loops.push(PerfLine {
                line: call.line,
                subject: call.base.clone(),
                frame,
            });
        }
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

fn call_fact(lineage: Lineage, src: &[u8]) -> Option<CallFact> {
    let node = lineage.node();
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
    call.region = control_region(lineage);
    call.offset = node.start_byte();
    call.awaited = node
        .parent()
        .is_some_and(|parent| parent.kind() == "await_expression");
    Some(call)
}

fn iteration_call(lineage: Lineage, src: &[u8], call: &CallFact) -> Option<CallFact> {
    let node = lineage.node();
    if !call.member {
        return None;
    }
    let pass = match call.method.as_str() {
        "map" if is_trivial_projection(node, src) => IterationPass::Projection,
        method if FULL_PASS_METHODS.contains(&method) => IterationPass::Full,
        _ => return None,
    };
    Some(CallFact {
        pass,
        in_assertion: in_assertion(lineage, src),
        arg_idents: node
            .child_by_field_name("arguments")
            .map(|args| super::loops::names(Some(args), src))
            .unwrap_or_default(),
        ..call.clone()
    })
}

/// `xs.map(x => x.field)`.
fn is_trivial_projection(call: Node, src: &[u8]) -> bool {
    let Some(arrow) = call
        .child_by_field_name("arguments")
        .filter(|args| args.named_child_count() == 1)
        .and_then(|args| args.named_child(0))
        .filter(|arg| arg.kind() == "arrow_function")
    else {
        return false;
    };
    let param = arrow
        .child_by_field_name("parameter")
        .or_else(|| sole_parameter(arrow))
        .and_then(|p| walk::node_text(p, src));
    let object = arrow
        .child_by_field_name("body")
        .filter(|body| body.kind() == "member_expression")
        .and_then(|body| body.child_by_field_name("object"))
        .and_then(|object| walk::node_text(object, src));
    param.is_some() && param == object
}

/// `p` in `(p) => …`.
fn sole_parameter(arrow: Node) -> Option<Node> {
    arrow
        .child_by_field_name("parameters")
        .filter(|params| params.named_child_count() == 1)
        .and_then(|params| params.named_child(0))
        .map(|param| param.child_by_field_name("pattern").unwrap_or(param))
}

fn in_assertion(lineage: Lineage, src: &[u8]) -> bool {
    let mut current = lineage.node();
    while let Some(parent) = lineage.parent(current) {
        if super::units::is_function(parent.kind()) {
            return false;
        }
        if parent.kind() == "call_expression" && is_assertion_call(parent, src) {
            return true;
        }
        current = parent;
    }
    false
}

/// `expect(..)`, `expect(..).toBe(..)`, `assert(..)`, `assert.equal(..)`.
fn is_assertion_call(call: Node, src: &[u8]) -> bool {
    let mut callee = call.child_by_field_name("function");
    while let Some(node) = callee {
        callee = match node.kind() {
            "member_expression" => node.child_by_field_name("object"),
            "call_expression" => node.child_by_field_name("function"),
            "identifier" => {
                return walk::node_text(node, src)
                    .is_some_and(|name| ASSERTION_CALLEES.contains(&name))
            }
            _ => None,
        };
    }
    false
}

fn record_bindings(facts: &mut PerformanceFacts, declarator: Node, src: &[u8]) {
    let end = declarator.end_byte();
    if let Some(name) = declarator.child_by_field_name("name") {
        facts.bindings.extend(
            super::loops::names(Some(name), src)
                .into_iter()
                .map(|ident| (ident, end)),
        );
    }
}

fn stable_receiver(node: &Node<'_>) -> bool {
    matches!(node.kind(), "identifier" | "this")
        || (node.kind() == "member_expression"
            && node
                .child_by_field_name("object")
                .is_some_and(|base| stable_receiver(&base)))
}

fn control_region(lineage: Lineage) -> usize {
    let mut current = lineage.node();
    while let Some(parent) = lineage.parent(current) {
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

pub fn loop_line(node: Node, src: &[u8]) -> PerfLine {
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

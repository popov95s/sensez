//! Python AST facts for performance-oriented smells.

use crate::profiles::walk;
use crate::spine::ir::{CallFact, PerfLine, PerformanceFacts};
use tree_sitter::Node;

const ITER_FUNCTIONS: [&str; 8] = ["any", "all", "sum", "min", "max", "sorted", "list", "set"];
const ITER_METHODS: [&str; 4] = ["count", "index", "copy", "join"];

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
    match func.kind() {
        "identifier" => Some(CallFact::named(walk::node_text(func, src)?, line)),
        "attribute" => {
            let base = func
                .child_by_field_name("object")
                .filter(|n| n.kind() == "identifier")
                .and_then(|n| walk::node_text(n, src))?;
            let method = func
                .child_by_field_name("attribute")
                .and_then(|n| walk::node_text(n, src))?;
            Some(CallFact::member(base, method, line))
        }
        _ => None,
    }
}

fn iteration_call(node: Node, src: &[u8], call: &CallFact) -> Option<CallFact> {
    if call.member && ITER_METHODS.contains(&call.method.as_str()) {
        return Some(call.clone());
    }
    if !call.member && ITER_FUNCTIONS.contains(&call.method.as_str()) {
        let base = first_arg_ident(node, src)?;
        return Some(CallFact {
            base: base.to_string(),
            ..call.clone()
        });
    }
    None
}

fn first_arg_ident<'a>(call: Node, src: &'a [u8]) -> Option<&'a str> {
    let args = call.child_by_field_name("arguments")?;
    let mut cursor = args.walk();
    let result = args
        .named_children(&mut cursor)
        .find(|n| n.kind() == "identifier")
        .and_then(|n| walk::node_text(n, src));
    result
}

fn line(node: Node, src: &[u8]) -> PerfLine {
    walk::perf_line(node, src, &["right"])
}

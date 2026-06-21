//! JS/TS AST facts for performance-oriented smells.

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

pub fn scan(facts: &mut PerformanceFacts, node: Node, src: &[u8], loop_depth: usize) {
    let kind = node.kind();
    if is_loop(kind) {
        facts.loops.push(line(node));
        if loop_depth > 0 {
            facts.nested_loops.push(line(node));
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
        facts.sorts_in_loops.push(PerfLine { line: call.line });
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
    match func.kind() {
        "identifier" => Some(CallFact::named(text(func, src)?, line)),
        "member_expression" => {
            let base = func
                .child_by_field_name("object")
                .filter(|n| matches!(n.kind(), "identifier" | "this"))
                .and_then(|n| text(n, src))?;
            let method = func
                .child_by_field_name("property")
                .and_then(|n| text(n, src))?;
            Some(CallFact::member(base, method, line))
        }
        _ => None,
    }
}

fn text<'a>(node: Node, src: &'a [u8]) -> Option<&'a str> {
    node.utf8_text(src).ok()
}

fn line(node: Node) -> PerfLine {
    PerfLine {
        line: node.start_position().row + 1,
    }
}

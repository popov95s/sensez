//! JS/TS collection passes counted by `repeated_iteration`.

use super::performance::{control_region, stable_receiver};
use crate::profiles::lineage::Lineage;
use crate::profiles::node_names::NodeNames;
use crate::profiles::walk;
use crate::spine::ir::{CallFact, IterationPass};
use tree_sitter::Node;

const PASS_METHODS: &[&str] = &[
    "every",
    "filter",
    "find",
    "findIndex",
    "flatMap",
    "forEach",
    "map",
    "reduce",
    "reduceRight",
    "some",
];
const AGGREGATE_METHODS: &[&str] = &["reduce", "reduceRight"];
const INDEX_CONSTRUCTORS: &[&str] = &["Map", "Set", "WeakMap", "WeakSet"];
const JSX_KINDS: &[&str] = &["jsx_element", "jsx_self_closing_element", "jsx_fragment"];
const ASSERTION_CALLEES: &[&str] = &["expect", "assert"];
const ASSIGNMENT_KINDS: &[&str] = &["assignment_expression", "augmented_assignment_expression"];

pub(super) fn iteration_call(lineage: Lineage, src: &[u8], call: &CallFact) -> Option<CallFact> {
    let node = lineage.node();
    if !call.member || builds_index(lineage, src) {
        return None;
    }
    Some(CallFact {
        pass: pass_kind(lineage, src, &call.method)?,
        in_assertion: in_assertion(lineage, src),
        arg_idents: node
            .field("arguments")
            .map(|args| super::loops::names(Some(args), src))
            .unwrap_or_default(),
        ..call.clone()
    })
}

/// `for (const x of xs)` as one loop pass over `xs`.
pub(super) fn loop_pass(lineage: Lineage, src: &[u8]) -> Option<CallFact> {
    let node = lineage.node();
    if node.kind_name() != "for_in_statement" {
        return None;
    }
    let base = node
        .field("right")
        .filter(stable_receiver)
        .and_then(|right| walk::node_text(right, src))?;
    let operator = node
        .field("operator")
        .and_then(|op| walk::node_text(op, src))
        .unwrap_or("of");
    let mut pass = CallFact::member(
        base,
        &format!("for-{operator}"),
        node.start_position().row + 1,
    );
    pass.pass = IterationPass::Loop;
    pass.region = control_region(lineage);
    pass.offset = node.start_byte();
    pass.arg_idents = node
        .field("body")
        .map(|body| read_only_names(body, src))
        .unwrap_or_default();
    Some(pass)
}

fn pass_kind(lineage: Lineage, src: &[u8], method: &str) -> Option<IterationPass> {
    let node = lineage.node();
    match method {
        "map" if is_trivial_projection(node, src) && is_reduced(lineage, src) => {
            Some(IterationPass::Aggregate)
        }
        "map" if is_trivial_projection(node, src) || renders_markup(node) => {
            Some(IterationPass::Projection)
        }
        method
            if AGGREGATE_METHODS.contains(&method) || is_counted_filter(lineage, src, method) =>
        {
            Some(IterationPass::Aggregate)
        }
        method if PASS_METHODS.contains(&method) => Some(IterationPass::Full),
        _ => None,
    }
}

/// `xs.map(x => x.n).reduce(..)`.
fn is_reduced(call: Lineage, src: &[u8]) -> bool {
    call.parent(call.node())
        .filter(|parent| parent.kind_name() == "member_expression")
        .and_then(|parent| parent.field("property"))
        .and_then(|property| walk::node_text(property, src))
        .is_some_and(|method| AGGREGATE_METHODS.contains(&method))
}

/// `xs.map((x) => <li>{x}</li>)`: rendering a list, not computing over it.
fn renders_markup(call: Node) -> bool {
    let body = call
        .field("arguments")
        .and_then(|args| args.named_child(0))
        .filter(|arg| arg.kind_name() == "arrow_function")
        .and_then(|arrow| arrow.field("body"));
    let markup = body.map(|body| match body.kind_name() {
        "parenthesized_expression" => body.named_child(0).unwrap_or(body),
        _ => body,
    });
    markup.is_some_and(|node| JSX_KINDS.contains(&node.kind_name()))
}

/// `xs.filter(..).length`.
fn is_counted_filter(call: Lineage, src: &[u8], method: &str) -> bool {
    method == "filter"
        && call
            .parent(call.node())
            .filter(|parent| parent.kind_name() == "member_expression")
            .and_then(|parent| parent.field("property"))
            .and_then(|property| walk::node_text(property, src))
            == Some("length")
}

/// `new Set(xs.map(..))`, `new Map(..)`, `Object.fromEntries(xs.map(..))`.
fn builds_index(call: Lineage, src: &[u8]) -> bool {
    let Some(owner) = call
        .parent(call.node())
        .filter(|parent| parent.kind_name() == "arguments")
        .and_then(|args| call.parent(args))
    else {
        return false;
    };
    let callee = match owner.kind_name() {
        "new_expression" => owner.field("constructor"),
        "call_expression" => owner.field("function"),
        _ => None,
    }
    .and_then(|callee| walk::node_text(callee, src));
    callee.is_some_and(|name| INDEX_CONSTRUCTORS.contains(&name) || name == "Object.fromEntries")
}

/// Names a loop body reads without assigning them.
fn read_only_names(body: Node, src: &[u8]) -> Vec<String> {
    let assigned = assigned_names(body, src);
    super::loops::names(Some(body), src)
        .into_iter()
        .filter(|name| !assigned.contains(name))
        .collect()
}

fn assigned_names(node: Node, src: &[u8]) -> Vec<String> {
    let target = match node.kind_name() {
        kind if ASSIGNMENT_KINDS.contains(&kind) => node.field("left"),
        "update_expression" => node.field("argument"),
        _ => None,
    };
    let mut cursor = node.walk();
    let nested: Vec<String> = node
        .named_children(&mut cursor)
        .flat_map(|child| assigned_names(child, src))
        .collect();
    target
        .filter(|target| target.kind_name() == "identifier")
        .and_then(|target| walk::node_text(target, src))
        .map(str::to_string)
        .into_iter()
        .chain(nested)
        .collect()
}

/// `xs.map(x => x.field)`.
fn is_trivial_projection(call: Node, src: &[u8]) -> bool {
    let Some(arrow) = call
        .field("arguments")
        .filter(|args| args.named_child_count() == 1)
        .and_then(|args| args.named_child(0))
        .filter(|arg| arg.kind_name() == "arrow_function")
    else {
        return false;
    };
    let param = arrow
        .field("parameter")
        .or_else(|| sole_parameter(arrow))
        .and_then(|p| walk::node_text(p, src));
    let object = arrow
        .field("body")
        .filter(|body| body.kind_name() == "member_expression")
        .and_then(|body| body.field("object"))
        .and_then(|object| walk::node_text(object, src));
    param.is_some() && param == object
}

/// `p` in `(p) => …`.
fn sole_parameter(arrow: Node) -> Option<Node> {
    arrow
        .field("parameters")
        .filter(|params| params.named_child_count() == 1)
        .and_then(|params| params.named_child(0))
        .map(|param| param.field("pattern").unwrap_or(param))
}

fn in_assertion(lineage: Lineage, src: &[u8]) -> bool {
    let mut current = lineage.node();
    while let Some(parent) = lineage.parent(current) {
        if super::units::is_function(parent.kind_name()) {
            return false;
        }
        if parent.kind_name() == "call_expression" && is_assertion_call(parent, src) {
            return true;
        }
        current = parent;
    }
    false
}

/// `expect(..)`, `expect(..).toBe(..)`, `assert(..)`, `assert.equal(..)`.
fn is_assertion_call(call: Node, src: &[u8]) -> bool {
    let mut callee = call.field("function");
    while let Some(node) = callee {
        callee = match node.kind_name() {
            "member_expression" => node.field("object"),
            "call_expression" => node.field("function"),
            "identifier" => {
                return walk::node_text(node, src)
                    .is_some_and(|name| ASSERTION_CALLEES.contains(&name))
            }
            _ => None,
        };
    }
    false
}

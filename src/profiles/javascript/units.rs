//! Per-function / per-class structural metrics for the design-smell pillar
//! (JavaScript/TypeScript). Mirrors `lang::python::units`: each function's
//! metrics cover its own body only — nested functions/arrows get their own
//! [`FunctionUnit`] — and `this.<attr>` is normalized to the canonical `"self"`
//! receiver so the language-neutral detectors work unchanged.

use super::{conditionals, obsession, performance, symbols};
use crate::spine::ir::FunctionUnit;
use tree_sitter::Node;

/// Numeric literals that are never "magic".
const ALLOWED_NUMS: [&str; 5] = ["0", "1", "2", "0.0", "1.0"];

/// Every JS/TS node kind that opens its own function scope.
pub fn is_function(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "function_expression"
            | "function"
            | "arrow_function"
            | "generator_function"
            | "generator_function_declaration"
            | "method_definition"
    )
}

/// Build a [`FunctionUnit`] for a function/method/arrow node.
pub fn analyze_function(func: Node, src: &[u8], is_method: bool) -> FunctionUnit {
    let mut unit = FunctionUnit {
        name: symbols::def_name(func, src).unwrap_or_default(),
        start_line: func.start_position().row + 1,
        end_line: func.end_position().row + 1,
        param_names: param_names(func, src),
        is_method,
        ..Default::default()
    };
    if let Some(body) = func.child_by_field_name("body") {
        let mut acc = Acc { unit: &mut unit };
        acc.visit(body, src, 0, 0);
    }
    // A TS tuple *return type* `(): [A, B, C]` is position-based grouped data —
    // the analog of Python's bare `return a, b, c` (JS array returns are too
    // common to treat as tuples, so only the annotation counts here).
    unit.max_tuple_return = super::classunit::tuple_return_arity(func, src);
    unit
}

/// Ordered parameter names; a destructuring pattern counts as one opaque param.
fn param_names(func: Node, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(p) = func.child_by_field_name("parameter") {
        push_param(p, src, &mut out); // single-identifier arrow
    }
    if let Some(params) = func.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        for p in params.named_children(&mut cursor) {
            push_param(p, src, &mut out);
        }
    }
    out
}

fn push_param(p: Node, src: &[u8], out: &mut Vec<String>) {
    let pat = match p.kind() {
        "required_parameter" | "optional_parameter" => p.child_by_field_name("pattern"),
        _ => Some(p),
    };
    if let Some(name) = pat.and_then(|n| pattern_name(n, src)) {
        out.push(name);
    }
}

fn pattern_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            node.utf8_text(src).ok().map(str::to_string)
        }
        "assignment_pattern" => node
            .child_by_field_name("left")
            .and_then(|l| pattern_name(l, src)),
        "rest_pattern" => node.named_child(0).and_then(|c| pattern_name(c, src)),
        "object_pattern" | "array_pattern" => Some("{}".to_string()),
        _ => None,
    }
}

/// Body-walk accumulator (borrows the unit so helpers stay small).
struct Acc<'u> {
    unit: &'u mut FunctionUnit,
}

impl Acc<'_> {
    /// Recurse a body node at block-nesting `depth`, accumulating metrics.
    fn visit(&mut self, node: Node, src: &[u8], depth: usize, loop_depth: usize) {
        let kind = node.kind();
        if is_function(kind) {
            return; // nested function/arrow gets its own unit
        }
        obsession::scan(self.unit, node, src);
        performance::scan(&mut self.unit.performance, node, src, loop_depth);

        let mut child_depth = depth;
        if is_nesting(kind) {
            child_depth = depth + 1;
            self.unit.max_nesting = self.unit.max_nesting.max(child_depth);
        }
        let child_loop_depth = loop_depth + usize::from(performance::is_loop(kind));
        if let Some(weight) = cognitive_weight(kind, node, src, depth) {
            self.unit.cognitive += weight;
        }
        if is_branch(kind, node, src) {
            self.unit.branch_count += 1;
        }
        if conditionals::is_collapsible_nested_if(node) {
            self.unit.collapsible_nested_ifs += 1;
        }
        match kind {
            "return_statement" => self.unit.return_count += 1,
            "number" => {
                if let Ok(t) = node.utf8_text(src) {
                    if !ALLOWED_NUMS.contains(&t) {
                        self.unit.magic_numbers += 1;
                    }
                }
            }
            "member_expression" => self.record_member(node, src),
            "assignment_expression" => self.record_assignment(node, src),
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(child, src, child_depth, child_loop_depth);
        }
    }

    /// `obj.attr` / `this.attr`: chain depth + receiver access (`this` → `self`).
    fn record_member(&mut self, node: Node, src: &[u8]) {
        self.unit.max_chain_depth = self.unit.max_chain_depth.max(chain_len(node));
        let Some(obj) = node.child_by_field_name("object") else {
            return;
        };
        match obj.kind() {
            "this" => {
                *self
                    .unit
                    .receiver_access
                    .entry("self".to_string())
                    .or_insert(0) += 1;
                if let Some(attr) = node
                    .child_by_field_name("property")
                    .and_then(|a| a.utf8_text(src).ok())
                {
                    self.unit.self_attrs.insert(attr.to_string());
                }
            }
            "identifier" => {
                if let Ok(base) = obj.utf8_text(src) {
                    *self
                        .unit
                        .receiver_access
                        .entry(base.to_string())
                        .or_insert(0) += 1;
                }
            }
            _ => {}
        }
    }

    /// Count a plain assignment to each simple-identifier target.
    fn record_assignment(&mut self, node: Node, src: &[u8]) {
        if let Some(left) = node
            .child_by_field_name("left")
            .filter(|l| l.kind() == "identifier")
            .and_then(|l| l.utf8_text(src).ok())
        {
            *self
                .unit
                .local_reassigns
                .entry(left.to_string())
                .or_insert(0) += 1;
        }
    }
}

/// Length of the pure attribute chain `a.b.c.d` ending at this `member_expression`
/// — does not traverse through call results, so fluent APIs aren't mistaken for
/// data navigation (same rule as the Python walk).
fn chain_len(node: Node) -> usize {
    let base = match node.child_by_field_name("object") {
        Some(obj) if obj.kind() == "member_expression" => chain_len(obj),
        _ => 0,
    };
    base + 1
}

/// Kinds that increase block-nesting depth.
fn is_nesting(kind: &str) -> bool {
    matches!(
        kind,
        "if_statement"
            | "for_statement"
            | "for_in_statement"
            | "while_statement"
            | "do_statement"
            | "switch_statement"
            | "try_statement"
    )
}

/// `&&` / `||` / `??` short-circuit operators add a decision point.
fn is_logical(node: Node, src: &[u8]) -> bool {
    node.child_by_field_name("operator")
        .and_then(|o| o.utf8_text(src).ok())
        .is_some_and(|op| matches!(op, "&&" | "||" | "??"))
}

/// Cyclomatic decision points.
fn is_branch(kind: &str, node: Node, src: &[u8]) -> bool {
    match kind {
        "if_statement" | "for_statement" | "for_in_statement" | "while_statement"
        | "do_statement" | "catch_clause" | "switch_case" | "ternary_expression" => true,
        "binary_expression" => is_logical(node, src),
        _ => false,
    }
}

/// Cognitive-complexity increment (Sonar-style): control structures cost
/// `1 + nesting`; logical operators a flat `1`.
fn cognitive_weight(kind: &str, node: Node, src: &[u8], depth: usize) -> Option<usize> {
    match kind {
        "if_statement" | "for_statement" | "for_in_statement" | "while_statement"
        | "do_statement" | "switch_statement" | "ternary_expression" | "catch_clause" => {
            Some(1 + depth)
        }
        "binary_expression" if is_logical(node, src) => Some(1),
        _ => None,
    }
}

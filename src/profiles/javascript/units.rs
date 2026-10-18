//! Per-function / per-class structural metrics for the design-smell pillar
//! (JavaScript/TypeScript). Mirrors `lang::python::units`: each function's
//! metrics cover its own body only — nested functions/arrows get their own
//! [`FunctionUnit`] — and `this.<attr>` is normalized to the canonical `"self"`
//! receiver so the language-neutral detectors work unchanged.

use super::chains::{chain_len, chain_root, is_chain_inner};
use super::member_visibility::{function_visibility, method_visibility};
use super::nesting::{cognitive_weight, is_branch, is_nesting};
use super::params::param_names;
use super::{conditionals, obsession, performance, symbols};
use crate::profiles::lineage::Lineage;
use crate::spine::ir::{bump, CallbackPosition, ChainHit, FunctionUnit, Walked};
use std::collections::{HashMap, HashSet};
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

pub struct FunctionFacts {
    pub(crate) unit: FunctionUnit,
    guards: HashMap<u64, usize>,
    body_start: usize,
    body_end: usize,
    depth: usize,
    loop_depth: usize,
}

impl FunctionFacts {
    pub fn start(lineage: Lineage, src: &[u8], is_method: bool) -> Self {
        let func = lineage.node();
        let mut unit = FunctionUnit {
            name: symbols::function_name(lineage, src).unwrap_or_default(),
            start_line: func.start_position().row + 1,
            end_line: func.end_position().row + 1,
            param_names: param_names(func, src),
            is_method,
            max_tuple_return: super::classunit::tuple_return_arity(func, src),
            callback_position: callback_position(func),
            visibility: if is_method {
                method_visibility(func, src)
            } else {
                function_visibility(func)
            },
            ..Default::default()
        };
        if !is_method && func.kind() == "function_declaration" {
            unit.review_risks.forwards_to =
                super::risk_facts::forward_target(func, src, &unit.param_names);
        }
        unit.review_risks.is_test_case = super::risk_facts::is_test_callback(func, src);
        let body = func.child_by_field_name("body");
        let range = body.map(|b| b.byte_range()).unwrap_or(0..0);
        FunctionFacts {
            unit,
            guards: HashMap::new(),
            body_start: range.start,
            body_end: range.end,
            depth: 0,
            loop_depth: 0,
        }
    }

    pub fn enter(&mut self, lineage: Lineage, src: &[u8]) -> Option<(usize, usize)> {
        let node = lineage.node();
        let range = node.byte_range();
        if range.start < self.body_start || range.end > self.body_end {
            return None;
        }
        if is_function(node.kind()) {
            self.unit.review_risks.test_has_nested_function = true;
            return None;
        }
        let prev_depth = self.depth;
        let prev_loop_depth = self.loop_depth;

        obsession::scan(&mut self.unit, node, src);
        performance::scan(&mut self.unit.performance, lineage, src, self.loop_depth);
        if let Some(method) = own_method_call(node, src) {
            self.unit.own_method_calls.insert(method);
        }
        super::risk_facts::scan(&mut self.unit, &mut self.guards, node, src);

        let kind = node.kind();
        if is_nesting(kind) {
            self.depth += 1;
            self.unit.max_nesting = self.unit.max_nesting.max(self.depth);
        }
        if performance::is_loop(kind) {
            self.loop_depth += 1;
        }
        if let Some(weight) = cognitive_weight(kind, node, src, prev_depth) {
            self.unit.cognitive += weight;
        }
        if is_branch(kind, node, src) {
            self.unit.branch_count += 1;
        }
        if conditionals::is_collapsible_nested_if(node) {
            self.unit
                .collapsible_nested_if_lines
                .push(node.start_position().row + 1);
        }
        match kind {
            "ternary_expression" => self
                .unit
                .nested_ternaries
                .extend(conditionals::nested_ternary(node)),
            "return_statement" => {
                self.unit.return_count += 1;
                if let Some(constructor) = returned_constructor(node, src) {
                    self.unit.returned_constructors.insert(constructor);
                }
                if let Some(name) = returned_name(node, src) {
                    self.unit.returned_names.insert(name);
                }
            }
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
        Some((self.depth - prev_depth, self.loop_depth - prev_loop_depth))
    }

    pub fn leave(&mut self, frame: (usize, usize)) {
        self.depth -= frame.0;
        self.loop_depth -= frame.1;
    }

    pub fn finish(self) -> FunctionUnit {
        self.unit
    }
}

impl FunctionFacts {
    fn record_member(&mut self, node: Node, src: &[u8]) {
        if !is_chain_inner(node) {
            self.unit.chain_hits.push(ChainHit {
                line: node.start_position().row + 1,
                depth: chain_len(node),
                root: chain_root(node, src),
            });
        }
        let Some(obj) = node.child_by_field_name("object") else {
            return;
        };
        match obj.kind() {
            "this" => {
                bump(&mut self.unit.receiver_access, "self");
                if let Some(attr) = node
                    .child_by_field_name("property")
                    .and_then(|a| a.utf8_text(src).ok())
                {
                    self.unit.self_attrs.insert(attr.to_string());
                }
            }
            "identifier" => {
                if let Ok(base) = obj.utf8_text(src) {
                    bump(&mut self.unit.receiver_access, base);
                }
            }
            _ => {}
        }
    }

    fn record_assignment(&mut self, node: Node, src: &[u8]) {
        if let Some(left) = node
            .child_by_field_name("left")
            .filter(|l| l.kind() == "identifier")
            .and_then(|l| l.utf8_text(src).ok())
        {
            bump(&mut self.unit.local_reassigns, left);
        }
    }
}

/// A hop through an `import * as ns` binding is a module path, not navigation.
pub fn discount_namespace_hops(out: &mut Walked) {
    let namespaces: HashSet<&str> = out
        .symbols
        .imports
        .iter()
        .flat_map(|import| import.imported_symbols.iter().zip(&import.bindings))
        .filter(|(symbol, _)| *symbol == "*")
        .map(|(_, binding)| binding.as_str())
        .collect();
    if namespaces.is_empty() {
        return;
    }
    for hit in out
        .units
        .functions
        .iter_mut()
        .flat_map(|f| f.chain_hits.iter_mut())
    {
        if hit.root.as_deref().is_some_and(|r| namespaces.contains(r)) {
            hit.depth = hit.depth.saturating_sub(1);
        }
    }
}

fn own_method_call(node: Node<'_>, src: &[u8]) -> Option<String> {
    if node.kind() != "call_expression" {
        return None;
    }
    let function = node.child_by_field_name("function")?;
    if function.kind() != "member_expression"
        || function
            .child_by_field_name("object")
            .is_none_or(|object| object.kind() != "this")
    {
        return None;
    }
    function
        .child_by_field_name("property")
        .and_then(|method| method.utf8_text(src).ok())
        .map(str::to_string)
}

fn returned_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    let value = node.named_child(0).filter(|v| v.kind() == "identifier")?;
    value.utf8_text(src).ok().map(str::to_string)
}

/// Callee name when `func` is passed directly as a call argument.
fn callback_position(func: Node) -> CallbackPosition {
    let is_argument = func
        .parent()
        .filter(|p| p.kind() == "arguments")
        .and_then(|args| args.parent())
        .is_some_and(|call| call.kind() == "call_expression");
    match is_argument {
        true => CallbackPosition::Argument,
        false => CallbackPosition::Standalone,
    }
}

fn returned_constructor(node: Node<'_>, src: &[u8]) -> Option<String> {
    let value = node.named_child(0)?;
    let function = match value.kind() {
        "call_expression" => value.child_by_field_name("function"),
        "new_expression" => value.child_by_field_name("constructor"),
        _ => None,
    }?;
    (function.kind() == "identifier")
        .then(|| function.utf8_text(src).ok().map(str::to_string))
        .flatten()
}

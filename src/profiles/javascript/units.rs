//! Per-function / per-class structural metrics for the design-smell pillar
//! (JavaScript/TypeScript). Mirrors `lang::python::units`: each function's
//! metrics cover its own body only — nested functions/arrows get their own
//! [`FunctionUnit`] — and `this.<attr>` is normalized to the canonical `"self"`
//! receiver so the language-neutral detectors work unchanged.

use super::chains::CHAIN;
use super::member_visibility::{callback_position, function_visibility, method_visibility};
use super::nesting::{cognitive_weight, is_branch, is_else_if, is_nesting};
use super::params::param_names;
use super::{
    classunit, conditionals, control_shapes, loops, magic_numbers, obsession, performance, symbols,
};
use crate::profiles::branch_path::branch_path;
use crate::profiles::guard_clause::is_guard_return;
use crate::profiles::guard_fingerprint::GuardLog;
use crate::profiles::lexeme::{self, BoundNames};
use crate::profiles::lineage::Lineage;
use crate::profiles::magic_number::{default_values, magic_number};
use crate::profiles::walk;
use crate::spine::ir::{bump, FunctionUnit};
use rustc_hash::FxHashMap;
use tree_sitter::Node;

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

/// What one `enter` pushed, for the matching `leave` to undo.
pub struct Entered {
    depth: usize,
    frames: usize,
}

pub struct FunctionFacts {
    pub(crate) unit: FunctionUnit,
    guards: GuardLog,
    body_start: usize,
    body_end: usize,
    depth: usize,
    frames: Vec<usize>,
    /// Node id of a per-iteration loop part → its loop frame.
    loop_parts: FxHashMap<usize, usize>,
    /// Parent-scope names whose read flags `captures_parent`.
    pub(super) outer_names: BoundNames,
}

impl FunctionFacts {
    pub fn start(lineage: Lineage, src: &[u8], is_method: bool) -> Self {
        let func = lineage.node();
        let tuple_return = classunit::tuple_return_elements(func, src);
        let mut unit = FunctionUnit {
            name: symbols::function_name(lineage, src).unwrap_or_default(),
            start_line: func.start_position().row + 1,
            end_line: func.end_position().row + 1,
            param_names: param_names(func, src),
            is_method,
            max_tuple_return: classunit::tuple_return_arity(&tuple_return),
            callback_position: callback_position(lineage),
            visibility: if is_method {
                method_visibility(func, src)
            } else {
                function_visibility(lineage)
            },
            homogeneous_tuple_return: classunit::is_homogeneous_primitive_tuple(&tuple_return),
            magic_numbers: default_values(func, src, &magic_numbers::SYNTAX),
            ..Default::default()
        };
        if !is_method && func.kind() == "function_declaration" {
            unit.review_risks.forwards_to =
                super::risk_facts::forward_target(func, src, &unit.param_names);
        }
        unit.review_risks.is_test_case = super::risk_facts::is_test_callback(lineage, src);
        unit.review_risks.stub = super::stub_body::stub_body(func, src);
        unit.is_constructor = is_method && unit.name == "constructor";
        let body = func.child_by_field_name("body");
        let range = body.map(|b| b.byte_range()).unwrap_or(0..0);
        let frames = loops::iteration_callback_frame(lineage, src)
            .map(|frame| unit.performance.open_frame(frame))
            .into_iter()
            .collect();
        FunctionFacts {
            unit,
            guards: GuardLog::new(super::wording::WORDING.identifier_extra_chars),
            body_start: range.start,
            body_end: range.end,
            depth: 0,
            frames,
            loop_parts: FxHashMap::default(),
            outer_names: BoundNames::default(),
        }
    }

    pub fn enter(&mut self, lineage: Lineage, src: &[u8]) -> Option<Entered> {
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
        let frames = self.enter_loop_part(node);

        obsession::scan(&mut self.unit, node, src);
        let frame = self.frame();
        performance::scan(&mut self.unit.performance, lineage, src, frame);
        if let Some(method) = own_method_call(node, src) {
            self.unit.own_method_calls.insert(method);
        }
        super::risk_facts::scan(&mut self.unit, &mut self.guards, lineage, src, frame);

        let kind = node.kind();
        if is_nesting(kind) && !is_else_if(lineage) {
            self.depth += 1;
            self.unit.max_nesting = self.unit.max_nesting.max(self.depth);
        }
        if performance::is_loop(kind) {
            let frame = loops::loop_frame(node, src, self.frame());
            let line = performance::loop_line(node, src);
            let id = self.unit.performance.open_loop(line, frame);
            self.loop_parts.extend(
                loops::iterated_parts(node)
                    .into_iter()
                    .map(|part| (part.id(), id)),
            );
        }
        if let Some(weight) = cognitive_weight(kind, lineage, src, prev_depth) {
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
                if is_guard_return(lineage, &control_shapes::GUARD_CLAUSE) {
                    self.unit.guard_returns += 1;
                }
                if let Some(constructor) = returned_constructor(node, src) {
                    self.unit.returned_constructors.insert(constructor);
                }
                if let Some(name) = returned_name(node, src) {
                    self.unit.returned_names.insert(name);
                }
            }
            "number" => {
                self.unit
                    .magic_numbers
                    .extend(magic_number(lineage, src, &magic_numbers::SYNTAX))
            }
            "member_expression" => self.record_member(node, lineage.parent(node), src),
            "assignment_expression" => self.record_assignment(lineage, src),
            "identifier" | "shorthand_property_identifier" => self.record_capture(node, src),
            _ => {}
        }
        Some(Entered {
            depth: self.depth - prev_depth,
            frames,
        })
    }

    pub fn leave(&mut self, entered: Entered) {
        self.depth -= entered.depth;
        self.frames.truncate(self.frames.len() - entered.frames);
    }

    pub fn finish(self) -> FunctionUnit {
        self.unit
    }
}

impl FunctionFacts {
    fn frame(&self) -> Option<usize> {
        self.frames.last().copied()
    }

    /// Open the enclosing loop's frame when `node` runs once per iteration.
    /// Push the frame of a loop part `node` starts; returns how many frames were pushed.
    fn enter_loop_part(&mut self, node: Node) -> usize {
        let Some(&id) = self.loop_parts.get(&node.id()) else {
            return 0;
        };
        self.frames.push(id);
        1
    }

    fn record_member(&mut self, node: Node, parent: Option<Node>, src: &[u8]) {
        self.unit
            .chain_hits
            .extend(walk::chain_hit(node, parent, src, &CHAIN));
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

    fn record_assignment(&mut self, lineage: Lineage, src: &[u8]) {
        let node = lineage.node();
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        if is_own_member(left) {
            self.unit.self_writes += 1;
        }
        let frame = self.frame();
        let paging = !matches!(left.kind(), "member_expression" | "subscript_expression")
            && node.child_by_field_name("right").is_some_and(|value| {
                walk::fed_by_call(&self.unit.performance, frame, value, src, "call_expression")
            });
        if paging {
            let names = loops::names(Some(left), src);
            self.unit.performance.note_advanced(frame, &names);
        }
        if let Some(name) = Some(left)
            .filter(|left| left.kind() == "identifier")
            .and_then(|left| left.utf8_text(src).ok())
        {
            self.unit
                .local_reassigns
                .entry(name.to_string())
                .or_default()
                .push(branch_path(lineage, &control_shapes::BRANCH_ARMS));
        }
    }

    fn record_capture(&mut self, node: Node, src: &[u8]) {
        if node
            .utf8_text(src)
            .is_ok_and(|name| self.outer_names.contains(&lexeme::hash(name)))
        {
            self.unit.captures_parent = true;
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

fn is_own_member(node: Node) -> bool {
    node.kind() == "member_expression"
        && node
            .child_by_field_name("object")
            .is_some_and(|object| object.kind() == "this")
}

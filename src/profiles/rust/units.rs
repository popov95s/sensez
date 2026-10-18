//! Per-function structural metrics for Rust smells.

use super::control_shapes;
use crate::profiles::branch_path::branch_path;
use crate::profiles::guard_clause::is_guard_return;
use crate::profiles::walk;
use crate::spine::ir::{
    FallbackSink, FunctionKey, FunctionUnit, LiteralMembership, PerfLine, TypeHints,
};
use tree_sitter::Node;

use super::params::param_names;
use super::unit_helpers::{
    call_fact, cognitive_weight, is_branch, is_loop, is_nesting, is_string, method_call,
    pattern_name, target_root, tuple_type_arity, type_text, unquote, CHAIN,
};

const ITER_METHODS: &[&str] = &["fold", "any", "all", "count", "sum", "position"];
const MUTATORS: &[&str] = &[
    "push",
    "pop",
    "insert",
    "remove",
    "clear",
    "sort",
    "sort_by",
    "sort_unstable",
    "retain",
    "extend",
    "append",
    "truncate",
    "swap_remove",
];

pub fn analyze_function(
    func: Node,
    src: &[u8],
    is_method: bool,
    hints: &mut TypeHints,
) -> FunctionUnit {
    let key = FunctionKey {
        name: super::symbols::def_name(func, src).unwrap_or_default(),
        start_line: func.start_position().row + 1,
    };
    let mut unit = FunctionUnit {
        name: key.name.clone(),
        start_line: key.start_line,
        end_line: func.end_position().row + 1,
        param_names: param_names(func, src, &key, hints),
        is_method,
        ..Default::default()
    };
    if let Some(ret) = func
        .child_by_field_name("return_type")
        .and_then(|n| type_text(n, src))
    {
        hints.record_return(key, ret.clone());
        unit.max_tuple_return = tuple_type_arity(&ret);
    }
    if let Some(body) = func.child_by_field_name("body") {
        let mut acc = Acc { unit: &mut unit };
        acc.visit(body, src, 0, None);
    }
    unit
}

struct Acc<'a> {
    unit: &'a mut FunctionUnit,
}

impl Acc<'_> {
    fn visit(&mut self, node: Node, src: &[u8], depth: usize, frame: Option<usize>) {
        let kind = node.kind();
        if matches!(kind, "function_item" | "closure_expression") {
            return;
        }
        let loop_frame = is_loop(kind).then(|| {
            let opened = super::loops::loop_frame(node, src, frame);
            let line = walk::perf_line(node, src, &["value", "condition"]);
            self.unit.performance.open_loop(line, opened)
        });
        self.scan(node, src, frame);

        let child_depth = next_depth(kind, depth);
        if child_depth > depth {
            self.unit.max_nesting = self.unit.max_nesting.max(child_depth);
        }
        if is_branch(kind, node, src) {
            self.unit.branch_count += 1;
        }
        if let Some(weight) = cognitive_weight(kind, depth) {
            self.unit.cognitive += weight;
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            let child_frame = match loop_frame {
                Some(id) if walk::is_field_child(node, child, &["body"]) => Some(id),
                _ => frame,
            };
            self.visit(child, src, child_depth, child_frame);
        }
    }

    fn scan(&mut self, node: Node, src: &[u8], frame: Option<usize>) {
        match node.kind() {
            "return_expression" => {
                self.unit.return_count += 1;
                if is_guard_return(node, &control_shapes::GUARD_CLAUSE) {
                    self.unit.guard_returns += 1;
                }
                self.record_tuple_return(node);
            }
            "let_declaration" => self.record_let(node, src, frame),
            "assignment_expression" | "compound_assignment_expr" => {
                self.record_assignment(node, src, frame)
            }
            "field_expression" => self.record_field(node, src),
            "index_expression" => {
                self.record_string_key(node, src);
                self.record_index_mutation(node, src);
            }
            "call_expression" => self.record_call(node, src, frame),
            "match_expression" => self.record_literal_match(node, src),
            _ => {}
        }
    }

    fn record_let(&mut self, node: Node, src: &[u8], frame: Option<usize>) {
        if let Some(pattern) = node.child_by_field_name("pattern") {
            let names = walk::identifiers(pattern, src, &["identifier"]);
            self.unit.performance.note_declared(frame, &names);
            if node.child_by_field_name("value").is_some_and(|value| {
                walk::fed_by_call(&self.unit.performance, frame, value, src, "call_expression")
            }) {
                self.unit.performance.note_call_results(frame, &names);
            }
        }
        if let Some(pat) = node
            .child_by_field_name("pattern")
            .and_then(|p| pattern_name(p, src))
        {
            self.record_local(pat, node);
        }
    }

    fn record_local(&mut self, name: String, node: Node) {
        self.unit
            .local_reassigns
            .entry(name)
            .or_default()
            .push(branch_path(node, &control_shapes::BRANCH_ARMS));
    }

    fn record_assignment(&mut self, node: Node, src: &[u8], frame: Option<usize>) {
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        if let Some(name) = pattern_name(left, src) {
            let paging = node.kind() == "assignment_expression"
                && node.child_by_field_name("right").is_some_and(|value| {
                    walk::fed_by_call(&self.unit.performance, frame, value, src, "call_expression")
                });
            if paging {
                self.unit
                    .performance
                    .note_advanced(frame, std::slice::from_ref(&name));
            }
            self.record_local(name, node);
        }
        walk::record_mutation_root(self.unit, left, src, target_root);
    }

    fn record_field(&mut self, node: Node, src: &[u8]) {
        self.unit
            .chain_hits
            .extend(walk::chain_hit(node, src, &CHAIN));
        let Some(base) = node
            .child_by_field_name("value")
            .filter(|n| matches!(n.kind(), "identifier" | "self"))
            .and_then(|n| n.utf8_text(src).ok())
        else {
            return;
        };
        *self
            .unit
            .receiver_access
            .entry(base.to_string())
            .or_insert(0) += 1;
        if base == "self" {
            if let Some(field) = node
                .child_by_field_name("field")
                .and_then(|n| n.utf8_text(src).ok())
            {
                self.unit.self_attrs.insert(field.to_string());
            }
        }
    }

    fn record_call(&mut self, node: Node, src: &[u8], frame: Option<usize>) {
        self.record_literal_contains(node, src);
        let Some(mut call) = call_fact(node, src) else {
            return;
        };
        self.unit.performance.calls.push(call.clone());
        if call.member && ITER_METHODS.contains(&call.method.as_str()) {
            self.unit.performance.iteration_calls.push(call.clone());
        }
        if call.member && MUTATORS.contains(&call.method.as_str()) {
            if let Some(func) = node.child_by_field_name("function") {
                if let Some(base) = func.child_by_field_name("value") {
                    walk::record_mutation_root(self.unit, base, src, target_root);
                }
            }
        }
        if call.member && matches!(call.method.as_str(), "unwrap_or" | "unwrap_or_default") {
            self.record_short_string_arg(node, src);
        }
        if frame.is_none() {
            return;
        }
        call.frame = frame;
        if call.member && call.method.starts_with("sort") {
            self.unit.performance.sorts_in_loops.push(PerfLine {
                line: call.line,
                subject: call.base.clone(),
                frame,
            });
        } else {
            self.unit.performance.loop_calls.push(call);
        }
    }

    fn record_string_key(&mut self, node: Node, src: &[u8]) {
        let recv = node.named_child(0).filter(|n| n.kind() == "identifier");
        let key = node.named_child(1).filter(|n| is_string(n.kind()));
        if let (Some(recv), Some(key)) = (recv, key) {
            if let (Ok(recv), Ok(key)) = (recv.utf8_text(src), key.utf8_text(src)) {
                self.unit
                    .str_keys
                    .entry(recv.to_string())
                    .or_default()
                    .insert(unquote(key));
                walk::record_str_key_line(self.unit, recv, node.start_position().row + 1);
            }
        }
    }

    fn record_index_mutation(&mut self, node: Node, src: &[u8]) {
        if node.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                "assignment_expression" | "compound_assignment_expr"
            )
        }) {
            walk::record_mutation_root(self.unit, node, src, target_root);
        }
    }

    fn record_tuple_return(&mut self, node: Node) {
        if let Some(value) = node
            .named_child(0)
            .filter(|n| n.kind() == "tuple_expression")
        {
            self.unit.max_tuple_return = self.unit.max_tuple_return.max(value.named_child_count());
        }
    }

    /// `[..].contains(x)` or `[..].iter().any(..)` on an array literal.
    fn record_literal_contains(&mut self, call: Node, src: &[u8]) {
        let Some((method, receiver)) = method_call(call, src) else {
            return;
        };
        let (array, operand) = match method {
            "contains" => (
                receiver,
                call.child_by_field_name("arguments")
                    .and_then(|args| args.named_child(0)),
            ),
            "any" => match method_call(receiver, src) {
                Some(("iter", inner)) => (inner, None),
                _ => return,
            },
            _ => return,
        };
        if array.kind() == "array_expression" {
            self.push_literal_membership(call, operand, array, src);
        }
    }

    fn record_literal_match(&mut self, node: Node, src: &[u8]) {
        if node.children(&mut node.walk()).any(|c| is_string(c.kind())) {
            self.push_literal_membership(node, node.child_by_field_name("value"), node, src);
        }
    }

    fn push_literal_membership(&mut self, at: Node, operand: Option<Node>, list: Node, src: &[u8]) {
        let mut cursor = list.walk();
        let literals: Vec<String> = list
            .children(&mut cursor)
            .filter(|c| is_string(c.kind()))
            .filter_map(|c| c.utf8_text(src).ok())
            .map(unquote)
            .collect();
        if literals.is_empty() {
            return;
        }
        self.unit.literal_memberships.push(LiteralMembership {
            line: at.start_position().row + 1,
            operand: operand
                .and_then(|o| o.utf8_text(src).ok())
                .map(|text| text.trim_start_matches('&').to_string()),
            literals,
        });
    }

    fn record_short_string_arg(&mut self, node: Node, src: &[u8]) {
        let Some(args) = node.child_by_field_name("arguments") else {
            return;
        };
        let Some(arg) = args.named_child(0).filter(|n| is_string(n.kind())) else {
            return;
        };
        walk::record_string_fallback(
            self.unit,
            arg.utf8_text(src).ok().map(unquote),
            FallbackSink::Unclassified,
            arg.start_position().row + 1,
        );
    }
}

fn next_depth(kind: &str, depth: usize) -> usize {
    depth + usize::from(is_nesting(kind))
}

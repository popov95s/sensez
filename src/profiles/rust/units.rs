//! Per-function structural metrics for Rust smells.

use crate::profiles::walk;
use crate::spine::ir::{FunctionUnit, TypeHints};
use tree_sitter::Node;

use super::unit_helpers::{
    call_fact, chain_len, cognitive_weight, is_branch, is_loop, is_nesting, is_string,
    pattern_name, target_root, tuple_type_arity, type_text, unquote,
};

const ITER_METHODS: &[&str] = &[
    "iter",
    "iter_mut",
    "into_iter",
    "map",
    "filter",
    "fold",
    "any",
    "all",
];
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
    let name = super::symbols::def_name(func, src).unwrap_or_default();
    let mut unit = FunctionUnit {
        name: name.clone(),
        start_line: func.start_position().row + 1,
        end_line: func.end_position().row + 1,
        param_names: param_names(func, src, &name, hints),
        is_method,
        ..Default::default()
    };
    if let Some(ret) = func
        .child_by_field_name("return_type")
        .and_then(|n| type_text(n, src))
    {
        hints.return_types.insert(name.clone(), ret.clone());
        unit.max_tuple_return = tuple_type_arity(&ret);
    }
    if let Some(body) = func.child_by_field_name("body") {
        let mut acc = Acc { unit: &mut unit };
        acc.visit(body, src, 0, 0);
    }
    unit
}

fn param_names(func: Node, src: &[u8], func_name: &str, hints: &mut TypeHints) -> Vec<String> {
    let Some(params) = func.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut cursor = params.walk();
    for param in params.named_children(&mut cursor) {
        if param.kind() == "self_parameter" {
            continue;
        }
        let Some(pattern) = param.child_by_field_name("pattern") else {
            continue;
        };
        let name = pattern_name(pattern, src).unwrap_or_else(|| "{}".to_string());
        if let Some(ty) = param
            .child_by_field_name("type")
            .and_then(|n| type_text(n, src))
        {
            hints
                .param_types
                .insert((func_name.to_string(), name.clone()), ty);
        }
        out.push(name);
    }
    out
}

struct Acc<'a> {
    unit: &'a mut FunctionUnit,
}

impl Acc<'_> {
    fn visit(&mut self, node: Node, src: &[u8], depth: usize, loop_depth: usize) {
        let kind = node.kind();
        if matches!(kind, "function_item" | "closure_expression") {
            return;
        }
        let child_loop_depth = loop_depth + usize::from(is_loop(kind));
        self.scan(node, src, loop_depth);

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
            self.visit(child, src, child_depth, child_loop_depth);
        }
    }

    fn scan(&mut self, node: Node, src: &[u8], loop_depth: usize) {
        match node.kind() {
            "for_expression" | "while_expression" | "loop_expression" => {
                let line = walk::perf_line(node, src, &["value", "condition"]);
                self.unit.performance.loops.push(line.clone());
                if loop_depth > 0 {
                    self.unit.performance.nested_loops.push(line);
                }
            }
            "return_expression" => {
                self.unit.return_count += 1;
                self.record_tuple_return(node);
            }
            "let_declaration" => self.record_let(node, src),
            "assignment_expression" | "compound_assignment_expr" => {
                self.record_assignment(node, src)
            }
            "field_expression" => self.record_field(node, src),
            "index_expression" => {
                self.record_string_key(node, src);
                self.record_index_mutation(node, src);
            }
            "call_expression" => self.record_call(node, src, loop_depth),
            "match_expression" => self.record_literal_match(node),
            _ => {}
        }
    }

    fn record_let(&mut self, node: Node, src: &[u8]) {
        if let Some(pat) = node
            .child_by_field_name("pattern")
            .and_then(|p| pattern_name(p, src))
        {
            *self.unit.local_reassigns.entry(pat).or_insert(0) += 1;
        }
    }

    fn record_assignment(&mut self, node: Node, src: &[u8]) {
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        if let Some(name) = pattern_name(left, src) {
            *self.unit.local_reassigns.entry(name).or_insert(0) += 1;
        }
        walk::record_mutation_root(self.unit, left, src, target_root);
    }

    fn record_field(&mut self, node: Node, src: &[u8]) {
        self.unit.max_chain_depth = self.unit.max_chain_depth.max(chain_len(node));
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

    fn record_call(&mut self, node: Node, src: &[u8], loop_depth: usize) {
        if self.record_literal_contains(node) {
            self.unit.literal_membership_tests += 1;
        }
        let Some(call) = call_fact(node, src) else {
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
        if loop_depth == 0 {
            return;
        }
        if call.member && call.method.starts_with("sort") {
            self.unit
                .performance
                .sorts_in_loops
                .push(walk::perf_line(node, src, &["value"]));
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

    fn record_literal_contains(&self, call: Node) -> bool {
        call.child_by_field_name("function")
            .filter(|f| f.kind() == "field_expression")
            .and_then(|f| f.child_by_field_name("value"))
            .is_some_and(|v| v.kind() == "array_expression")
    }

    fn record_literal_match(&mut self, node: Node) {
        if node.children(&mut node.walk()).any(|c| is_string(c.kind())) {
            self.unit.literal_membership_tests += 1;
        }
    }

    fn record_short_string_arg(&mut self, node: Node, src: &[u8]) {
        let Some(args) = node.child_by_field_name("arguments") else {
            return;
        };
        let Some(arg) = args.named_child(0).filter(|n| is_string(n.kind())) else {
            return;
        };
        walk::record_short_string_fallback(
            self.unit,
            arg.utf8_text(src).ok().map(|s| unquote(s).chars().count()),
            arg.start_position().row + 1,
        );
    }
}

fn next_depth(kind: &str, depth: usize) -> usize {
    depth + usize::from(is_nesting(kind))
}

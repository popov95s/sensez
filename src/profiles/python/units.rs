//! Per-function / per-class structural metrics for the design-smell pillar.
//!
//! Nested functions/lambdas get their own unit, so metrics do not double-count.

use super::{classunit, conditionals, symbols};
use crate::spine::ir::{ClassUnit, FunctionUnit};
use tree_sitter::Node;

/// Numeric literals that are never "magic".
const ALLOWED_NUMS: [&str; 5] = ["0", "1", "2", "0.0", "1.0"];

/// Build a [`FunctionUnit`] for a `function_definition` node.
pub fn analyze_function(func: Node, src: &[u8], is_method: bool) -> FunctionUnit {
    let mut unit = FunctionUnit {
        name: symbols::def_name(func, src).unwrap_or_default(),
        start_line: func.start_position().row + 1,
        end_line: func.end_position().row + 1,
        param_names: symbols::param_names(func, src),
        is_method,
        ..Default::default()
    };
    if let Some(body) = func.child_by_field_name("body") {
        let mut acc = Acc::new(&mut unit);
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            acc.visit(child, src, 0);
        }
    }
    unit
}

/// Body-walk accumulator. Borrows the unit so helpers stay small.
struct Acc<'u> {
    unit: &'u mut FunctionUnit,
}

impl<'u> Acc<'u> {
    fn new(unit: &'u mut FunctionUnit) -> Self {
        Acc { unit }
    }

    /// Recurse a body node at block-nesting `depth`, accumulating metrics.
    fn visit(&mut self, node: Node, src: &[u8], depth: usize) {
        let kind = node.kind();
        // Nested functions/lambdas get their own unit — do not descend.
        if matches!(kind, "function_definition" | "lambda") {
            return;
        }
        super::obsession::scan(self.unit, node, src);

        let mut child_depth = depth;
        if is_nesting(kind) {
            child_depth = depth + 1;
            self.unit.max_nesting = self.unit.max_nesting.max(child_depth);
        }
        if let Some(weight) = cognitive_weight(kind, depth) {
            self.unit.cognitive += weight;
        }
        if is_branch(kind) {
            self.unit.branch_count += 1;
        }
        if conditionals::is_collapsible_nested_if(node) {
            self.unit.collapsible_nested_ifs += 1;
        }
        match kind {
            "return_statement" => self.unit.return_count += 1,
            "integer" | "float" => {
                if let Ok(t) = node.utf8_text(src) {
                    if !ALLOWED_NUMS.contains(&t) {
                        self.unit.magic_numbers += 1;
                    }
                }
            }
            "attribute" => {
                self.unit.max_chain_depth = self.unit.max_chain_depth.max(chain_len(node));
                if let Some(base) = node
                    .child_by_field_name("object")
                    .filter(|o| o.kind() == "identifier")
                    .and_then(|o| o.utf8_text(src).ok())
                {
                    *self
                        .unit
                        .receiver_access
                        .entry(base.to_string())
                        .or_insert(0) += 1;
                    // Feed LCOM from this body walk instead of traversing each
                    // method again later.
                    if base == "self" {
                        if let Some(attr) = node
                            .child_by_field_name("attribute")
                            .and_then(|a| a.utf8_text(src).ok())
                        {
                            self.unit.self_attrs.insert(attr.to_string());
                        }
                    }
                }
            }
            "assignment" => self.record_assignment(node, src),
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(child, src, child_depth);
        }
    }

    /// Count a plain (non-augmented) assignment to each simple-identifier target.
    fn record_assignment(&mut self, node: Node, src: &[u8]) {
        for target in symbols::assignment_targets(node, src) {
            *self.unit.local_reassigns.entry(target).or_insert(0) += 1;
        }
    }
}

/// Build a [`ClassUnit`] for a `class_definition` node.
pub fn analyze_class(class: Node, src: &[u8]) -> ClassUnit {
    let mut unit = ClassUnit {
        name: symbols::def_name(class, src).unwrap_or_default(),
        start_line: class.start_position().row + 1,
        end_line: class.end_position().row + 1,
        bases: symbols::base_classes(class, src),
        properties: classunit::properties(class, src),
        ..Default::default()
    };
    // An ABC/Protocol base makes the class itself abstract.
    unit.is_abstract = unit.bases.iter().any(|b| is_abstract_base(b));

    let Some(body) = class.child_by_field_name("body") else {
        return unit;
    };
    let mut cursor = body.walk();
    for member in body.named_children(&mut cursor) {
        let (method, decorators) = match member.kind() {
            "function_definition" => (member, Vec::new()),
            "decorated_definition" => match member.child_by_field_name("definition") {
                Some(d) if d.kind() == "function_definition" => {
                    (d, symbols::decorator_paths(member, src))
                }
                _ => continue,
            },
            _ => continue,
        };
        let Some(name) = symbols::def_name(method, src) else {
            continue;
        };
        let is_abstract_method = decorators
            .iter()
            .any(|p| p.rsplit('.').next() == Some("abstractmethod"));
        if is_abstract_method {
            unit.is_abstract = true; // declaring an abstract method makes the class abstract
        }
        unit.methods.push(name.clone());
        if let Some(mbody) = method.child_by_field_name("body") {
            // A `@abstractmethod` stub is a correct declaration, not a refused bequest.
            if !is_abstract_method && is_stub_body(mbody, src) {
                unit.overrides_to_stub.push(name.clone());
            }
        }
        // `method_attr_use[name]` is filled by the post-walk join in `traversal`
        // from each method's own `FunctionUnit.self_attrs` — no second body walk.
    }
    unit
}

/// Base-class names that mark a class as abstract / interface-like.
fn is_abstract_base(base: &str) -> bool {
    matches!(
        base.rsplit('.').next(),
        Some("ABC") | Some("ABCMeta") | Some("Protocol")
    )
}

/// A method body is a "stub" if its only real statement is `pass` or
/// `raise NotImplementedError` (a leading docstring is ignored).
fn is_stub_body(body: Node, src: &[u8]) -> bool {
    let mut cursor = body.walk();
    let mut stmts: Vec<Node> = body.named_children(&mut cursor).collect();
    stmts.retain(|s| !is_docstring(*s, src));
    match stmts.as_slice() {
        [one] => match one.kind() {
            "pass_statement" => true,
            "raise_statement" => one
                .utf8_text(src)
                .map(|t| t.contains("NotImplementedError"))
                .unwrap_or(false),
            _ => false,
        },
        _ => false,
    }
}

fn is_docstring(node: Node, src: &[u8]) -> bool {
    node.kind() == "expression_statement"
        && node
            .named_child(0)
            .map(|c| matches!(c.kind(), "string" | "concatenated_string"))
            .unwrap_or(false)
        && node.utf8_text(src).is_ok()
}

/// Length of the pure attribute chain `a.b.c.d` ending at this `attribute` node.
///
/// Only consecutive attribute accesses count — the chain does **not** traverse
/// through `call` results, so fluent/builder APIs (`q.filter().limit().all()`)
/// are not mistaken for Law-of-Demeter data navigation.
fn chain_len(node: Node) -> usize {
    let base = match node.child_by_field_name("object") {
        Some(obj) if obj.kind() == "attribute" => chain_len(obj),
        _ => 0,
    };
    base + 1
}

/// Kinds that increase block-nesting depth.
fn is_nesting(kind: &str) -> bool {
    matches!(
        kind,
        "if_statement" | "for_statement" | "while_statement" | "try_statement" | "with_statement"
    )
}

/// Kinds counted as cyclomatic decision points.
fn is_branch(kind: &str) -> bool {
    matches!(
        kind,
        "if_statement"
            | "elif_clause"
            | "for_statement"
            | "while_statement"
            | "except_clause"
            | "boolean_operator"
            | "conditional_expression"
            | "if_clause"
    )
}

/// Cognitive-complexity increment (Sonar-style): control structures cost
/// `1 + nesting`; boolean operators and `elif` cost a flat `1`.
fn cognitive_weight(kind: &str, depth: usize) -> Option<usize> {
    match kind {
        "if_statement" | "for_statement" | "while_statement" | "conditional_expression" => {
            Some(1 + depth)
        }
        "except_clause" => Some(1 + depth),
        "elif_clause" | "boolean_operator" => Some(1),
        _ => None,
    }
}

//! Function-scope bound-name analysis for JS/TS duplication normalization.
//!
//! Mirrors the Python analysis: a name local to a function (its parameters, or
//! a `var`/`let`/`const` / `for` / `catch` binding in its body) is the only
//! thing a real clone may freely rename, so it collapses to one lexeme code.
//! Member/property names, free names, and literals are kept verbatim.

use std::collections::HashSet;
use tree_sitter::Node;

/// Names bound within `func` (parameters + body declarations), not descending
/// into nested function scopes.
pub fn bound_names(func: Node, src: &[u8]) -> HashSet<String> {
    let mut set = HashSet::new();
    // arrow_function may have a single `parameter` or a `parameters` list.
    if let Some(param) = func.child_by_field_name("parameter") {
        collect_pattern(param, src, &mut set);
    }
    if let Some(params) = func.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        for p in params.named_children(&mut cursor) {
            collect_pattern(p, src, &mut set);
        }
    }
    if let Some(body) = func.child_by_field_name("body") {
        collect_targets(body, src, &mut set);
    }
    set
}

/// Collect declaration/`for`/`catch` targets, skipping nested function scopes.
fn collect_targets(node: Node, src: &[u8], set: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "function_declaration"
            | "function_expression"
            | "function"
            | "arrow_function"
            | "generator_function"
            | "generator_function_declaration"
            | "method_definition" => {} // separate scope — don't descend
            "lexical_declaration" | "variable_declaration" => {
                let mut c = child.walk();
                for d in child.named_children(&mut c) {
                    if let Some(name) = d.child_by_field_name("name") {
                        collect_pattern(name, src, set);
                    }
                }
                collect_targets(child, src, set);
            }
            "for_in_statement" | "for_statement" => {
                if let Some(left) = child.child_by_field_name("left") {
                    collect_pattern(left, src, set);
                }
                collect_targets(child, src, set);
            }
            "catch_clause" => {
                if let Some(param) = child.child_by_field_name("parameter") {
                    collect_pattern(param, src, set);
                }
                collect_targets(child, src, set);
            }
            _ => collect_targets(child, src, set),
        }
    }
}

/// Names introduced by a binding pattern (identifier or destructuring).
fn collect_pattern(node: Node, src: &[u8], set: &mut HashSet<String>) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            if let Ok(t) = node.utf8_text(src) {
                set.insert(t.to_string());
            }
        }
        "required_parameter" | "optional_parameter" => {
            // TS parameter wrappers: the binding is the `pattern` field.
            if let Some(pat) = node.child_by_field_name("pattern") {
                collect_pattern(pat, src, set);
            }
        }
        "array_pattern" | "object_pattern" | "rest_pattern" | "assignment_pattern"
        | "pair_pattern" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_pattern(child, src, set);
            }
        }
        _ => {}
    }
}

//! Function-scope bound-name analysis for duplication normalization.
//!
//! A name is *bound-local* to a function if it's a parameter or is assigned in
//! the body (assignment / `for` / `with as` / walrus target). Those are the
//! only names a real clone may rename, so they collapse to a single token;
//! everything else (module/free names, attribute/method names, type
//! annotations, literals) is kept verbatim.

use std::collections::HashSet;
use tree_sitter::Node;

/// Names bound within `func` (its parameters + body targets), not descending
/// into nested functions/lambdas (those have their own scope).
pub fn bound_names(func: Node, src: &[u8]) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Some(params) = func.child_by_field_name("parameters") {
        collect_params(params, src, &mut set);
    }
    if let Some(body) = func.child_by_field_name("body") {
        collect_targets(body, src, &mut set);
    }
    set
}

fn collect_params(params: Node, src: &[u8], set: &mut HashSet<String>) {
    let mut cursor = params.walk();
    for p in params.named_children(&mut cursor) {
        let name = if p.kind() == "identifier" {
            text(p, src)
        } else {
            // typed_parameter / default_parameter / *args / **kwargs: the name
            // is the first identifier child (the type, if any, comes after).
            first_identifier(p, src)
        };
        if let Some(name) = name {
            set.insert(name);
        }
    }
}

/// Recursively collect assignment/`for`/walrus/`as` targets, skipping nested
/// function/lambda scopes.
fn collect_targets(node: Node, src: &[u8], set: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "function_definition" | "lambda" => {} // separate scope — don't descend
            "assignment" | "augmented_assignment" | "for_statement" => {
                if let Some(left) = child.child_by_field_name("left") {
                    collect_target_names(left, src, set);
                }
                collect_targets(child, src, set);
            }
            "named_expression" => {
                if let Some(name) = child.child_by_field_name("name") {
                    if let Some(t) = text(name, src) {
                        set.insert(t);
                    }
                }
                collect_targets(child, src, set);
            }
            "as_pattern" => {
                if let Some(alias) = child.child_by_field_name("alias") {
                    collect_target_names(alias, src, set);
                }
                collect_targets(child, src, set);
            }
            _ => collect_targets(child, src, set),
        }
    }
}

/// Names on the left of a binding. `self.x`/`arr[i]` targets are attribute or
/// subscript writes, not local bindings, so they are deliberately ignored.
fn collect_target_names(node: Node, src: &[u8], set: &mut HashSet<String>) {
    match node.kind() {
        "identifier" => {
            if let Some(t) = text(node, src) {
                set.insert(t);
            }
        }
        "pattern_list" | "tuple_pattern" | "list_pattern" | "tuple" | "list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_target_names(child, src, set);
            }
        }
        _ => {}
    }
}

fn first_identifier(node: Node, src: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return text(child, src);
        }
    }
    None
}

fn text(node: Node, src: &[u8]) -> Option<String> {
    node.utf8_text(src).ok().map(str::to_string)
}

//! Function-scope bound-name analysis for Rust duplication normalization.
//!
//! Mirrors the Python/JS analysis: a name local to a function (its parameters,
//! or a `let`/`for`/`if let` binding in its body) is the only thing a real
//! clone may freely rename, so it collapses to one lexeme code. Field names,
//! free names, types, and literals are kept verbatim.

use std::collections::HashSet;
use tree_sitter::Node;

/// Names bound within `func` (parameters + body bindings), not descending into
/// nested function/closure scopes.
pub fn bound_names(func: Node, src: &[u8]) -> HashSet<String> {
    let mut set = HashSet::new();
    // `function_item` has a `parameters` list; `closure_expression` has
    // `closure_parameters` (patterns directly as children).
    if let Some(params) = func.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        for p in params.named_children(&mut cursor) {
            match p.kind() {
                "parameter" => {
                    if let Some(pat) = p.child_by_field_name("pattern") {
                        collect_pattern(pat, src, &mut set);
                    }
                }
                "self_parameter" => {} // `self` is kept verbatim (API surface)
                _ => collect_pattern(p, src, &mut set), // closure patterns
            }
        }
    }
    if let Some(body) = func.child_by_field_name("body") {
        collect_targets(body, src, &mut set);
    }
    set
}

/// Collect `let`/`for`/`if let` binding targets, skipping nested fn scopes.
fn collect_targets(node: Node, src: &[u8], set: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "function_item" | "closure_expression" => {} // separate scope
            "let_declaration" | "let_condition" => {
                if let Some(pat) = child.child_by_field_name("pattern") {
                    collect_pattern(pat, src, set);
                }
                collect_targets(child, src, set);
            }
            "for_expression" => {
                if let Some(pat) = child.child_by_field_name("pattern") {
                    collect_pattern(pat, src, set);
                }
                collect_targets(child, src, set);
            }
            _ => collect_targets(child, src, set),
        }
    }
}

/// Names introduced by a binding pattern (identifier or destructuring). The
/// constructor path of `Some(x)` / `Point { x, y }` is a *type*, not a binding,
/// so the `type` field child is skipped.
fn collect_pattern(node: Node, src: &[u8], set: &mut HashSet<String>) {
    match node.kind() {
        "identifier" | "shorthand_field_identifier" => {
            if let Ok(t) = node.utf8_text(src) {
                set.insert(t.to_string());
            }
        }
        "tuple_struct_pattern" | "struct_pattern" => {
            let type_id = node.child_by_field_name("type").map(|t| t.id());
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if Some(child.id()) != type_id {
                    collect_pattern(child, src, set);
                }
            }
        }
        "tuple_pattern" | "slice_pattern" | "reference_pattern" | "mut_pattern" | "ref_pattern"
        | "or_pattern" | "field_pattern" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_pattern(child, src, set);
            }
        }
        _ => {}
    }
}

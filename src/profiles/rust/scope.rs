//! Function-scope bound-name analysis for Rust duplication normalization.

use crate::profiles::lexeme::BoundNames;
use crate::profiles::node_names::NodeNames;
use tree_sitter::Node;

/// Names bound within `func`, excluding nested function and closure scopes.
pub fn bound_names(func: Node, src: &[u8]) -> BoundNames {
    let mut set = BoundNames::default();
    if let Some(params) = func.field("parameters") {
        let mut cursor = params.walk();
        for p in params.named_children(&mut cursor) {
            match p.kind_name() {
                "parameter" => {
                    if let Some(pat) = p.field("pattern") {
                        collect_pattern(pat, src, &mut set);
                    }
                }
                "self_parameter" => {} // `self` is kept verbatim (API surface)
                _ => collect_pattern(p, src, &mut set), // closure patterns
            }
        }
    }
    if let Some(body) = func.field("body") {
        collect_targets(body, src, &mut set);
    }
    set
}

/// Collect `let`/`for`/`if let` binding targets, skipping nested fn scopes.
fn collect_targets(node: Node, src: &[u8], set: &mut BoundNames) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind_name() {
            "function_item" | "closure_expression" => {} // separate scope
            "let_declaration" | "let_condition" => {
                if let Some(pat) = child.field("pattern") {
                    collect_pattern(pat, src, set);
                }
                collect_targets(child, src, set);
            }
            "for_expression" => {
                if let Some(pat) = child.field("pattern") {
                    collect_pattern(pat, src, set);
                }
                collect_targets(child, src, set);
            }
            _ => collect_targets(child, src, set),
        }
    }
}

/// Names introduced by a binding pattern (identifier or destructuring).
fn collect_pattern(node: Node, src: &[u8], set: &mut BoundNames) {
    match node.kind_name() {
        "identifier" | "shorthand_field_identifier" => {
            if let Ok(t) = node.utf8_text(src) {
                set.insert(crate::profiles::lexeme::hash(t));
            }
        }
        "tuple_struct_pattern" | "struct_pattern" => {
            let type_id = node.field("type").map(|t| t.id());
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

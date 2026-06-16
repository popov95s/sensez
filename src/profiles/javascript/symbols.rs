//! Top-level declaration extraction for JavaScript/TypeScript.

use crate::spine::ir::SymbolKind;
use tree_sitter::Node;

/// Name of a `function_declaration` / `class_declaration` / `method_definition`.
pub fn def_name(node: Node, src: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Names declared by a `lexical_declaration` / `variable_declaration`, each with
/// a kind: `Function` when the initializer is a function/arrow, else `Variable`.
pub fn declared_vars(node: Node, src: &[u8]) -> Vec<(String, SymbolKind)> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|c| c.kind() == "variable_declarator")
        .filter_map(|d| {
            let name = d.child_by_field_name("name")?;
            if name.kind() != "identifier" {
                return None; // skip destructuring patterns
            }
            let text = name.utf8_text(src).ok()?.to_string();
            let kind = match d.child_by_field_name("value").map(|v| v.kind()) {
                Some("arrow_function" | "function" | "function_expression") => SymbolKind::Function,
                _ => SymbolKind::Variable,
            };
            Some((text, kind))
        })
        .collect()
}

//! Top-level declaration extraction for Rust.

use crate::profiles::node_names::NodeNames;
use crate::spine::ir::SymbolKind;
use tree_sitter::Node;

/// Name of a named item (`function_item`, `struct_item`, `trait_item`, ...).
pub fn def_name(node: Node, src: &[u8]) -> Option<String> {
    node.field("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Display name for a scope opener.
pub fn scope_name(node: Node, src: &[u8]) -> String {
    node.field("name")
        .or_else(|| node.field("type"))
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or(crate::spine::ir::ANONYMOUS)
        .to_string()
}

/// True if the item carries a visibility modifier (`pub`, `pub(crate)`, ...).
pub fn is_pub(node: Node) -> bool {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .any(|c| c.kind_name() == "visibility_modifier");
    found
}

/// The declared kind for a top-level item kind, if it declares a symbol.
pub fn declared_kind(kind: &str) -> Option<SymbolKind> {
    Some(match kind {
        "function_item" => SymbolKind::Function,
        "struct_item" | "enum_item" | "union_item" | "trait_item" | "type_item" => {
            SymbolKind::Class
        }
        "const_item" | "static_item" => SymbolKind::Variable,
        _ => return None,
    })
}

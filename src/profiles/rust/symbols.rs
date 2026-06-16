//! Top-level declaration extraction for Rust.
//!
//! Only `pub` items (any visibility scope: `pub`, `pub(crate)`, `pub(super)`)
//! are declared: rustc's own `dead_code` lint already owns private unreachable
//! items, so sensez complements it with what the compiler cannot see — public
//! items no module in the scan reaches.

use crate::spine::ir::SymbolKind;
use tree_sitter::Node;

/// Name of a named item (`function_item`, `struct_item`, `trait_item`, ...).
pub fn def_name(node: Node, src: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Display name for a scope opener: the `name` field, or the implemented type
/// for an `impl` block, or `<anon>` (closures).
pub fn scope_name(node: Node, src: &[u8]) -> String {
    node.child_by_field_name("name")
        .or_else(|| node.child_by_field_name("type"))
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or("<anon>")
        .to_string()
}

/// True if the item carries a visibility modifier (`pub`, `pub(crate)`, ...).
pub fn is_pub(node: Node) -> bool {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .any(|c| c.kind() == "visibility_modifier");
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

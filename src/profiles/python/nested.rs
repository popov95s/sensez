//! Facts about a function defined inside another function.

use super::scope;
use crate::profiles::lexeme::{self, BoundNames};
use crate::profiles::node_query::is_field_child;
use crate::spine::ir::FunctionUnit;
use tree_sitter::Node;

/// Record that `unit` sits in the function `enclosing`, whose bound names are `enclosing_names`.
pub(super) fn record(
    unit: &mut FunctionUnit,
    func: Node,
    src: &[u8],
    enclosing: &str,
    enclosing_names: Option<&BoundNames>,
) {
    unit.is_nested = true;
    unit.parent = enclosing.to_string();
    unit.parent_is_test_callback = enclosing.starts_with("test_");
    if let (Some(parent), Some(body)) = (enclosing_names, func.child_by_field_name("body")) {
        let own = scope::bound_names(func, src);
        let outer: BoundNames = parent.difference(&own).copied().collect();
        unit.captures_parent = reads_any(body, src, &outer);
    }
}

/// `node` reads one of `names` outside nested functions, attribute names and keyword names.
fn reads_any(node: Node, src: &[u8], names: &BoundNames) -> bool {
    match node.kind() {
        "function_definition" | "lambda" => return false,
        "identifier" => {
            return node
                .utf8_text(src)
                .is_ok_and(|name| names.contains(&lexeme::hash(name)))
        }
        _ => {}
    }
    let label_field: &[&str] = match node.kind() {
        "attribute" => &["attribute"],
        "keyword_argument" => &["name"],
        _ => &[],
    };
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| !is_field_child(node, child, label_field) && reads_any(child, src, names));
    found
}

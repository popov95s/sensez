//! Parameter names and type hints of a function node.

use super::unit_helpers::{pattern_name, type_text};
use crate::spine::ir::{FunctionKey, TypeHints};
use tree_sitter::Node;

pub(super) fn param_names(
    node: Node,
    src: &[u8],
    func: &FunctionKey,
    hints: &mut TypeHints,
) -> Vec<String> {
    let Some(params) = node.child_by_field_name("parameters") else {
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
            hints.record_param(func, &name, ty);
        }
        out.push(name);
    }
    out
}

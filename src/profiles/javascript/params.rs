//! Parameter names of a function node.

use crate::profiles::node_names::NodeNames;
use tree_sitter::Node;

pub(super) fn param_names(func: Node, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arrow_parameter) = func.field("parameter") {
        push_param(arrow_parameter, src, &mut out);
    }
    if let Some(params) = func.field("parameters") {
        let mut cursor = params.walk();
        for p in params.named_children(&mut cursor) {
            push_param(p, src, &mut out);
        }
    }
    out
}

fn push_param(p: Node, src: &[u8], out: &mut Vec<String>) {
    let pat = match p.kind_name() {
        "required_parameter" | "optional_parameter" => p.field("pattern"),
        _ => Some(p),
    };
    if let Some(name) = pat.and_then(|n| pattern_name(n, src)) {
        out.push(name);
    }
}

fn pattern_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind_name() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            node.utf8_text(src).ok().map(str::to_string)
        }
        "assignment_pattern" => node.field("left").and_then(|l| pattern_name(l, src)),
        "rest_pattern" => node.named_child(0).and_then(|c| pattern_name(c, src)),
        "object_pattern" | "array_pattern" => Some("{}".to_string()),
        _ => None,
    }
}

/// Names bound inside destructured parameters: `items` in `({ items }) => …`.
pub(super) fn destructured_params(func: Node, src: &[u8]) -> Vec<String> {
    let Some(params) = func.field("parameters") else {
        return Vec::new();
    };
    let mut cursor = params.walk();
    let patterns: Vec<Node> = params
        .named_children(&mut cursor)
        .filter_map(|p| p.field("pattern").or(Some(p)))
        .filter(|p| matches!(p.kind_name(), "object_pattern" | "array_pattern"))
        .collect();
    patterns
        .into_iter()
        .flat_map(|pattern| super::loops::names(Some(pattern), src))
        .collect()
}

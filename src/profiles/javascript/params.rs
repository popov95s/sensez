//! Parameter names of a function node.

use tree_sitter::Node;

pub(super) fn param_names(func: Node, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arrow_parameter) = func.child_by_field_name("parameter") {
        push_param(arrow_parameter, src, &mut out);
    }
    if let Some(params) = func.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        for p in params.named_children(&mut cursor) {
            push_param(p, src, &mut out);
        }
    }
    out
}

fn push_param(p: Node, src: &[u8], out: &mut Vec<String>) {
    let pat = match p.kind() {
        "required_parameter" | "optional_parameter" => p.child_by_field_name("pattern"),
        _ => Some(p),
    };
    if let Some(name) = pat.and_then(|n| pattern_name(n, src)) {
        out.push(name);
    }
}

fn pattern_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            node.utf8_text(src).ok().map(str::to_string)
        }
        "assignment_pattern" => node
            .child_by_field_name("left")
            .and_then(|l| pattern_name(l, src)),
        "rest_pattern" => node.named_child(0).and_then(|c| pattern_name(c, src)),
        "object_pattern" | "array_pattern" => Some("{}".to_string()),
        _ => None,
    }
}

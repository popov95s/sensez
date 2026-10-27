use crate::profiles::node_names::NodeNames;
use std::collections::HashMap;
use tree_sitter::Node;

pub(crate) fn evaluate(
    node: Node,
    source: &[u8],
    constants: &HashMap<String, String>,
) -> Option<String> {
    match node.kind_name() {
        "string" => unquote(text(node, source)?),
        "template_string" => template_pattern(text(node, source)?),
        "identifier" => constants.get(&text(node, source)?).cloned(),
        "parenthesized_expression" => node
            .named_child(0)
            .and_then(|child| evaluate(child, source, constants)),
        "binary_expression" => {
            let left = node.field("left")?;
            let right = node.field("right")?;
            Some(format!(
                "{}{}",
                evaluate(left, source, constants)?,
                evaluate(right, source, constants)?
            ))
        }
        _ => None,
    }
}

fn template_pattern(raw: String) -> Option<String> {
    let body = raw.strip_prefix('`')?.strip_suffix('`')?;
    let mut pattern = String::new();
    let mut rest = body;
    while let Some(start) = rest.find("${") {
        pattern.push_str(&rest[..start]);
        pattern.push('*');
        let tail = &rest[start + 2..];
        let end = tail.find('}')?;
        rest = &tail[end + 1..];
    }
    pattern.push_str(rest);
    Some(pattern)
}

fn unquote(raw: String) -> Option<String> {
    let first = raw.chars().next()?;
    let last = raw.chars().last()?;
    (matches!(first, '\'' | '"') && first == last)
        .then(|| raw[1..raw.len().saturating_sub(1)].to_string())
}

pub(crate) fn text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(str::to_string)
}

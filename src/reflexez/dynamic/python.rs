use super::{context, FileFacts};
use std::collections::HashMap;
use tree_sitter::Node;

pub fn scan(source: &[u8], module: &str) -> Option<FileFacts> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .ok()?;
    let tree = parser.parse(source, None)?;
    let constants = collect_constants(tree.root_node(), source);
    let mut facts = FileFacts::default();
    visit(tree.root_node(), source, module, &constants, &mut facts);
    (!facts.imports.is_empty() || !facts.patterns.is_empty() || facts.unresolved > 0)
        .then_some(facts)
}

fn collect_constants(root: Node, source: &[u8]) -> HashMap<String, String> {
    let mut bindings: HashMap<String, usize> = HashMap::new();
    count_binding_targets(root, source, &mut bindings);
    let mut constants = HashMap::new();
    collect_constant_nodes(root, source, &bindings, &mut constants);
    constants
}

fn is_simple_name(text: Option<String>) -> bool {
    text.is_some_and(|name| !name.contains(|ch: char| !ch.is_alphanumeric() && ch != '_'))
}

fn count_binding_targets(node: Node, source: &[u8], counts: &mut HashMap<String, usize>) {
    match node.kind() {
        "assignment" | "augmented_assignment" => {
            if let Some(name) = node.child_by_field_name("left").and_then(|n| text(n, source)) {
                if is_simple_name(Some(name.clone())) {
                    *counts.entry(name).or_default() += 1;
                }
            }
        }
        "for_statement" => {
            if let Some(name) = node.child_by_field_name("left").and_then(|n| text(n, source)) {
                if is_simple_name(Some(name.clone())) {
                    *counts.entry(name).or_default() += usize::MAX; // rebound every iteration
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        count_binding_targets(child, source, counts);
    }
}

fn collect_constant_nodes(
    node: Node,
    source: &[u8],
    bindings: &HashMap<String, usize>,
    out: &mut HashMap<String, String>,
) {
    if node.kind() == "assignment" {
        let name = node
            .child_by_field_name("left")
            .and_then(|n| text(n, source));
        if is_simple_name(name.clone()) && bindings.get(name.as_deref().unwrap_or("")) == Some(&1)
        {
            let value = node
                .child_by_field_name("right")
                .and_then(|n| evaluate(n, source, out));
            if let (Some(name), Some(value)) = (name, value) {
                out.insert(name, value);
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_constant_nodes(child, source, bindings, out);
    }
}

fn visit(
    node: Node,
    source: &[u8],
    module: &str,
    constants: &HashMap<String, String>,
    facts: &mut FileFacts,
) {
    if node.kind() == "call" {
        record_call(node, source, module, constants, facts);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(child, source, module, constants, facts);
    }
}

fn record_call(
    node: Node,
    source: &[u8],
    module: &str,
    constants: &HashMap<String, String>,
    facts: &mut FileFacts,
) {
    let name = node
        .child_by_field_name("function")
        .and_then(|function| text(function, source))
        .unwrap_or_default();
    if !matches!(name.as_str(), "importlib.import_module" | "__import__") {
        return;
    }
    let value = node
        .child_by_field_name("arguments")
        .and_then(|arguments| arguments.named_child(0))
        .and_then(|argument| evaluate(argument, source, constants));
    match value {
        Some(value) if value.contains('*') => facts.patterns.push(value),
        Some(value) => facts
            .imports
            .push(context(value, module, node.start_position().row + 1)),
        None => facts.unresolved += 1,
    }
}

fn evaluate(node: Node, source: &[u8], constants: &HashMap<String, String>) -> Option<String> {
    match node.kind() {
        "string" => python_string(text(node, source)?),
        "concatenated_string" => {
            let mut cursor = node.walk();
            let parts: Option<Vec<_>> = node
                .named_children(&mut cursor)
                .map(|child| evaluate(child, source, constants))
                .collect();
            Some(parts?.concat())
        }
        "identifier" => constants.get(&text(node, source)?).cloned(),
        "parenthesized_expression" => node
            .named_child(0)
            .and_then(|child| evaluate(child, source, constants)),
        "binary_operator" => {
            let left = node.child_by_field_name("left")?;
            let right = node.child_by_field_name("right")?;
            Some(format!(
                "{}{}",
                evaluate(left, source, constants)?,
                evaluate(right, source, constants)?
            ))
        }
        _ => None,
    }
}

fn python_string(raw: String) -> Option<String> {
    let quote_at = raw.find(['\'', '"'])?;
    let quote = raw.as_bytes()[quote_at] as char;
    let triple = raw[quote_at..].starts_with(&quote.to_string().repeat(3));
    let width = if triple { 3 } else { 1 };
    let start = quote_at + width;
    let end = raw.len().checked_sub(width)?;
    if start > end || !raw.ends_with(&quote.to_string().repeat(width)) {
        return None;
    }
    let body = &raw[start..end];
    if body.contains('{') && raw[..quote_at].to_ascii_lowercase().contains('f') {
        Some(fstring_pattern(body))
    } else {
        Some(body.to_string())
    }
}

fn fstring_pattern(body: &str) -> String {
    let mut pattern = String::new();
    let mut rest = body;
    while let Some(start) = rest.find('{') {
        pattern.push_str(&rest[..start]);
        pattern.push('*');
        let tail = &rest[start + 1..];
        let Some(end) = tail.find('}') else {
            return pattern;
        };
        rest = &tail[end + 1..];
    }
    pattern.push_str(rest);
    pattern
}

fn text(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_importlib_constant_and_pattern() {
        let source = br#"name = "pkg.feature"
importlib.import_module(name)
importlib.import_module(f"pkg.pages.{page}")
"#;
        let facts = scan(source, "sample").unwrap();
        assert_eq!(facts.imports[0].target_module, "pkg.feature");
        assert_eq!(facts.patterns, vec!["pkg.pages.*"]);
        assert_eq!(facts.unresolved, 0);
    }
}

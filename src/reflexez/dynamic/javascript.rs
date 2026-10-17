use super::{context, FileFacts};
use crate::profiles::javascript::const_eval::{evaluate, text};
use crate::profiles::javascript::grammar::language;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::Node;

pub fn scan(path: &Path, source: &[u8], module: &str) -> Option<FileFacts> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language(path)).ok()?;
    let tree = parser.parse(source, None)?;
    let constants = collect_constants(tree.root_node(), source);
    let mut facts = FileFacts::default();
    visit(tree.root_node(), source, module, &constants, &mut facts);
    (!facts.imports.is_empty() || !facts.patterns.is_empty() || facts.unresolved > 0)
        .then_some(facts)
}

fn collect_constants(root: Node, source: &[u8]) -> HashMap<String, String> {
    let mut constants = HashMap::new();
    collect_constant_nodes(root, source, &mut constants);
    constants
}

fn collect_constant_nodes(node: Node, source: &[u8], out: &mut HashMap<String, String>) {
    if node.kind() == "variable_declarator" && declarator_is_const(node, source) {
        let name = node
            .child_by_field_name("name")
            .and_then(|n| text(n, source));
        let value = node
            .child_by_field_name("value")
            .and_then(|n| evaluate(n, source, out));
        if let (Some(name), Some(value)) = (name, value) {
            out.insert(name, value);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_constant_nodes(child, source, out);
    }
}

fn declarator_is_const(declarator: Node, source: &[u8]) -> bool {
    declarator
        .parent()
        .and_then(|declaration| declaration.child_by_field_name("kind"))
        .and_then(|kind| kind.utf8_text(source).ok())
        .is_some_and(|keyword| keyword == "const")
}

fn visit(
    node: Node,
    source: &[u8],
    module: &str,
    constants: &HashMap<String, String>,
    facts: &mut FileFacts,
) {
    if node.kind() == "call_expression" {
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
    let Some(callee) = node.child_by_field_name("function") else {
        return;
    };
    let name = text(callee, source).unwrap_or_default();
    let dynamic = callee.kind() == "import" || name == "require";
    let glob = name == "import.meta.glob" || name == "import.meta.globEager";
    if !dynamic && !glob {
        return;
    }
    let argument = node
        .child_by_field_name("arguments")
        .and_then(|args| args.named_child(0));
    if let Some(array) = argument.filter(|arg| glob && arg.kind() == "array") {
        record_glob_array(array, source, constants, facts);
        return;
    }
    let value = argument.and_then(|arg| evaluate(arg, source, constants));
    match value {
        Some(value) if value.contains('*') => facts.patterns.push(vec![value]),
        Some(value) if glob => facts.patterns.push(vec![value]),
        Some(value) => {
            let literal_already_collected = argument.is_some_and(|arg| arg.kind() == "string");
            if !literal_already_collected {
                facts
                    .imports
                    .push(context(value, module, node.start_position().row + 1));
            }
        }
        None => facts.unresolved += 1,
    }
}

fn record_glob_array(
    array: Node,
    source: &[u8],
    constants: &HashMap<String, String>,
    facts: &mut FileFacts,
) {
    let mut patterns = Vec::new();
    let mut cursor = array.walk();
    for element in array
        .named_children(&mut cursor)
        .filter(|element| element.kind() != "comment")
    {
        match evaluate(element, source, constants) {
            Some(pattern) => patterns.push(pattern),
            None => facts.unresolved += 1,
        }
    }
    if !patterns.is_empty() {
        facts.patterns.push(patterns);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_constant_and_pattern_dynamic_imports() {
        let source =
            br#"const target = './feature'; import(target); import(`./pages/${name}.ts`);"#;
        let facts = scan(Path::new("sample.ts"), source, "sample").unwrap();
        assert_eq!(facts.imports[0].target_module, "./feature");
        assert_eq!(facts.patterns, vec![vec!["./pages/*.ts"]]);
        assert_eq!(facts.unresolved, 0);
    }

    #[test]
    fn resolves_array_glob_with_negation() {
        let source = br#"import.meta.glob(["./handlers/*.ts", "!./handlers/skip.ts"]);"#;
        let facts = scan(Path::new("registry.ts"), source, "registry").unwrap();
        assert_eq!(
            facts.patterns,
            vec![vec!["./handlers/*.ts", "!./handlers/skip.ts"]]
        );
        assert_eq!(facts.unresolved, 0);
    }
}

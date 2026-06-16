//! Python import-statement extraction into the shared [`ImportContext`].

use crate::spine::ir::ImportContext;
use tree_sitter::Node;

/// Extract zero or more imports from an `import_statement` /
/// `import_from_statement` node.
pub fn extract(
    node: Node,
    src: &[u8],
    source_module: &str,
    scope: Option<&str>,
) -> Vec<ImportContext> {
    let pos = node.start_position();
    let (line, column) = (pos.row + 1, pos.column + 1);
    let base = |target: String, symbols: Vec<String>, bindings: Vec<String>| ImportContext {
        source_module: source_module.to_string(),
        target_module: target,
        imported_symbols: symbols,
        bindings,
        line,
        column,
        is_inline: scope.is_some(),
        is_module_decl: false,
        enclosing_scope: scope.map(str::to_string),
    };

    match node.kind() {
        "import_statement" => import_targets(node, src)
            .into_iter()
            .map(|(target, bound)| base(target, Vec::new(), vec![bound]))
            .collect(),
        "import_from_statement" => {
            let (target, symbols, bindings) = from_import(node, src);
            vec![base(target, symbols, bindings)]
        }
        _ => Vec::new(),
    }
}

/// `import a.b, c as d` → [("a.b","a"), ("c","d")].
fn import_targets(node: Node, src: &[u8]) -> Vec<(String, String)> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter_map(|child| match child.kind() {
            "dotted_name" => {
                let module = text(child, src)?;
                let bound = top_component(&module);
                Some((module, bound))
            }
            "aliased_import" => {
                let module = child
                    .child_by_field_name("name")
                    .and_then(|n| text(n, src))?;
                let bound = child
                    .child_by_field_name("alias")
                    .and_then(|n| text(n, src))
                    .unwrap_or_else(|| top_component(&module));
                Some((module, bound))
            }
            _ => None,
        })
        .collect()
}

/// `from mod import a, b as c` → ("mod", ["a","b"], ["a","c"]). `*` → symbol "*".
fn from_import(node: Node, src: &[u8]) -> (String, Vec<String>, Vec<String>) {
    let target = node
        .child_by_field_name("module_name")
        .and_then(|n| text(n, src))
        .unwrap_or_default();
    let module_name_id = node.child_by_field_name("module_name").map(|n| n.id());

    let mut cursor = node.walk();
    let (mut symbols, mut bindings) = (Vec::new(), Vec::new());
    for c in node.named_children(&mut cursor) {
        if Some(c.id()) == module_name_id {
            continue;
        }
        match c.kind() {
            "dotted_name" | "identifier" => {
                if let Some(name) = text(c, src) {
                    symbols.push(name.clone());
                    bindings.push(name);
                }
            }
            "aliased_import" => {
                if let Some(name) = c.child_by_field_name("name").and_then(|n| text(n, src)) {
                    let alias = c.child_by_field_name("alias").and_then(|n| text(n, src));
                    bindings.push(alias.unwrap_or_else(|| name.clone()));
                    symbols.push(name);
                }
            }
            "wildcard_import" => symbols.push("*".to_string()),
            _ => {}
        }
    }
    (target, symbols, bindings)
}

fn top_component(module: &str) -> String {
    module.split('.').next().unwrap_or(module).to_string()
}

fn text(node: Node, src: &[u8]) -> Option<String> {
    node.utf8_text(src).ok().map(str::to_string)
}

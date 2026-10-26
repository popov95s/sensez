//! Top-level declaration extraction for JavaScript/TypeScript.

use crate::profiles::lineage::Lineage;
use crate::spine::ir::SymbolKind;
use tree_sitter::Node;

/// Name of a `function_declaration` / `class_declaration` / `method_definition`.
pub fn def_name(node: Node, src: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// True when `node` is the declaration of an `export default` statement.
pub fn is_default_export(node: Node) -> bool {
    node.parent().is_some_and(|parent| {
        parent.kind() == "export_statement"
            && parent
                .children(&mut parent.walk())
                .any(|child| child.kind() == "default")
    })
}

/// Name of a function/class node: its own name, the binding it is assigned to, or its call site.
pub fn function_name(lineage: Lineage, src: &[u8]) -> Option<String> {
    let node = lineage.node();
    if let Some(name) = def_name(node, src) {
        return Some(name);
    }
    let parent = lineage.parent(node)?;
    let field = match parent.kind() {
        "variable_declarator" | "public_field_definition" => "name",
        "field_definition" => "property",
        "pair" => return pair_key_name(parent.child_by_field_name("key")?, src),
        "assignment_expression" => "left",
        "arguments" => return callback_name(lineage, parent, src),
        _ => return None,
    };
    let text = parent.child_by_field_name(field)?.utf8_text(src).ok()?;
    Some(last_segment(text).to_string())
}

/// `{ save: ... }` and `{ "on-save": ... }` keys; computed keys have no name.
fn pair_key_name(key: Node, src: &[u8]) -> Option<String> {
    let text = key.utf8_text(src).ok()?;
    match key.kind() {
        "property_identifier" => Some(text.to_string()),
        "string" => Some(text.trim_matches(['"', '\'', '`']).to_string()),
        _ => None,
    }
}

/// `describe callback`, or `it("adds rows") callback` when a string title comes first.
fn callback_name(lineage: Lineage, args: Node, src: &[u8]) -> Option<String> {
    let call = lineage
        .parent(args)
        .filter(|p| p.kind() == "call_expression")?;
    let callee = call.child_by_field_name("function")?.utf8_text(src).ok()?;
    let callee = if callee.contains(['(', '\n']) {
        last_segment(callee)
    } else {
        callee
    };
    let title = args
        .named_child(0)
        .filter(|arg| arg.kind() == "string")
        .and_then(|arg| arg.utf8_text(src).ok());
    Some(match title {
        Some(title) => format!("{callee}({title}) callback"),
        None => format!("{callee} callback"),
    })
}

fn last_segment(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

/// Names declared by a `lexical_declaration` / `variable_declaration`, each with
/// a kind: `Function` when the initializer is a function/arrow, else `Variable`.
pub fn declared_vars(node: Node, src: &[u8]) -> Vec<(String, SymbolKind)> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|c| c.kind() == "variable_declarator")
        .filter_map(|d| {
            let name = d.child_by_field_name("name")?;
            if name.kind() != "identifier" {
                return None; // skip destructuring patterns
            }
            let text = name.utf8_text(src).ok()?.to_string();
            let kind = match d.child_by_field_name("value").map(|v| v.kind()) {
                Some("arrow_function" | "function" | "function_expression") => SymbolKind::Function,
                _ => SymbolKind::Variable,
            };
            Some((text, kind))
        })
        .collect()
}

/// `(name, callee)` for each `name = callee(...)` declarator, e.g. `("send", "fw.register")`.
pub fn initializer_callees(node: Node, src: &[u8]) -> Vec<(String, String)> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|c| c.kind() == "variable_declarator")
        .filter_map(|d| {
            let name = d.child_by_field_name("name")?;
            let value = d.child_by_field_name("value")?;
            if name.kind() != "identifier" || value.kind() != "call_expression" {
                return None;
            }
            let callee = value.child_by_field_name("function")?;
            if !matches!(callee.kind(), "identifier" | "member_expression") {
                return None;
            }
            Some((
                name.utf8_text(src).ok()?.to_string(),
                callee.utf8_text(src).ok()?.to_string(),
            ))
        })
        .collect()
}

/// Body of a `"…"` / `'…'` string literal.
pub fn quoted_string_value<'a>(node: Node, src: &'a [u8]) -> Option<&'a str> {
    let text = node.utf8_text(src).ok()?.trim();
    let quote = text.chars().next().filter(|ch| matches!(ch, '"' | '\''))?;
    let body = text.get(quote.len_utf8()..)?;
    Some(body.strip_suffix(quote).unwrap_or(body))
}

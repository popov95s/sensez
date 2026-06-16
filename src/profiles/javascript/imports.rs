//! JavaScript/TypeScript import extraction into the shared [`ImportContext`].
//!
//! Handles ES module `import`/`export … from` statements (including
//! re-export barrels and `export *`) and CommonJS `require("…")` calls. The
//! `target_module` is the raw specifier (`./foo`, `react`); resolution to a
//! module key happens later in `resolve`.

use crate::spine::ir::ImportContext;
use tree_sitter::Node;

/// Statement kinds carrying a module specifier we extract as an import.
pub fn is_import_statement(kind: &str) -> bool {
    matches!(kind, "import_statement" | "export_statement")
}

/// Extract imports from an `import_statement` / `export_statement` (the latter
/// only when it re-exports `from` a source).
pub fn extract(
    node: Node,
    src: &[u8],
    source_module: &str,
    scope: Option<&str>,
) -> Vec<ImportContext> {
    let source = match node
        .child_by_field_name("source")
        .and_then(|n| string_value(n, src))
    {
        Some(s) => s,
        None => return Vec::new(), // local `export { ... }` / `export const` — no edge
    };
    let (symbols, bindings) = clause_symbols(node, src);
    vec![context(
        source,
        symbols,
        bindings,
        node,
        source_module,
        scope,
    )]
}

/// Extract a CommonJS `require("…")` / dynamic `import("…")` call as an import,
/// if `node` is such a call. The bound local name is unknown here (assignment
/// happens in the enclosing declaration), so `bindings` stays empty.
pub fn require_import(
    node: Node,
    src: &[u8],
    source_module: &str,
    scope: Option<&str>,
) -> Option<ImportContext> {
    let callee = node.child_by_field_name("function")?;
    let name = callee.utf8_text(src).ok()?;
    if name != "require" && callee.kind() != "import" {
        return None;
    }
    let args = node.child_by_field_name("arguments")?;
    let mut cursor = args.walk();
    let source = args
        .named_children(&mut cursor)
        .find(|c| c.kind() == "string")
        .and_then(|s| string_value(s, src))?;
    Some(context(
        source,
        Vec::new(),
        Vec::new(),
        node,
        source_module,
        scope,
    ))
}

fn context(
    target: String,
    symbols: Vec<String>,
    bindings: Vec<String>,
    node: Node,
    source_module: &str,
    scope: Option<&str>,
) -> ImportContext {
    let pos = node.start_position();
    ImportContext {
        source_module: source_module.to_string(),
        target_module: target,
        imported_symbols: symbols,
        bindings,
        line: pos.row + 1,
        column: pos.column + 1,
        is_inline: scope.is_some(),
        is_module_decl: false,
        enclosing_scope: scope.map(str::to_string),
    }
}

/// Collect (imported symbols, local bindings) from an import/export clause.
/// `import Foo` → default; `* as ns` → namespace; `{ a, b as c }` → named;
/// `export * from` → symbol `*`.
fn clause_symbols(node: Node, src: &[u8]) -> (Vec<String>, Vec<String>) {
    let (mut symbols, mut bindings) = (Vec::new(), Vec::new());
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "import_clause" => {
                let (s, b) = clause_symbols(child, src);
                symbols.extend(s);
                bindings.extend(b);
            }
            "identifier" => {
                if let Ok(name) = child.utf8_text(src) {
                    symbols.push("default".to_string());
                    bindings.push(name.to_string());
                }
            }
            "namespace_import" => {
                if let Some(name) = first_identifier(child, src) {
                    symbols.push("*".to_string());
                    bindings.push(name);
                }
            }
            "named_imports" | "export_clause" => {
                collect_specifiers(child, src, &mut symbols, &mut bindings)
            }
            _ => {}
        }
    }
    // `export * from "x"` carries a bare `*` token, not a clause node.
    if node.kind() == "export_statement" && node.children(&mut node.walk()).any(|c| c.kind() == "*")
    {
        symbols.push("*".to_string());
    }
    (symbols, bindings)
}

fn collect_specifiers(
    clause: Node,
    src: &[u8],
    symbols: &mut Vec<String>,
    bindings: &mut Vec<String>,
) {
    let mut cursor = clause.walk();
    for spec in clause.named_children(&mut cursor) {
        if spec.kind() != "import_specifier" && spec.kind() != "export_specifier" {
            continue;
        }
        let name = spec
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(src).ok());
        let alias = spec
            .child_by_field_name("alias")
            .and_then(|n| n.utf8_text(src).ok());
        if let Some(name) = name {
            symbols.push(name.to_string());
            bindings.push(alias.unwrap_or(name).to_string());
        }
    }
}

fn first_identifier(node: Node, src: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|c| c.kind() == "identifier")
        .and_then(|c| c.utf8_text(src).ok())
        .map(str::to_string);
    found
}

/// Text of a string literal with surrounding quotes stripped.
fn string_value(node: Node, src: &[u8]) -> Option<String> {
    let raw = node.utf8_text(src).ok()?;
    let trimmed = raw
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '`');
    Some(trimmed.to_string())
}

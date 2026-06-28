//! Rust `use`-tree extraction into the shared [`ImportContext`].
//!
//! A `use` declaration is a tree (`use crate::a::{b, c::{d as e, self}, *};`)
//! that flattens to one [`ImportContext`] per leaf: the leaf's prefix path is
//! the `target_module` (raw, `::`-separated — resolution to a module key
//! happens later in `resolve`), the leaf itself is the imported symbol, and
//! the alias (if any) is the local binding. `pub use` re-exports are extracted
//! identically — the re-export *edge* is what keeps the target alive.

use crate::spine::ir::{ImportContext, ImportPhase};
use tree_sitter::Node;

/// Extract every leaf of a `use_declaration` as an [`ImportContext`].
pub fn extract(
    node: Node,
    src: &[u8],
    source_module: &str,
    scope: Option<&str>,
) -> Vec<ImportContext> {
    let mut out = Vec::new();
    if let Some(arg) = node.child_by_field_name("argument") {
        flatten(arg, src, &mut Vec::new(), &mut |prefix, leaf| {
            out.push(context(prefix, leaf, node, source_module, scope, false));
        });
    }
    out
}

/// A `mod name;` declaration (no body): an edge from the declaring module to
/// its child module, expressed as a `self::name` import.
pub fn mod_decl(
    node: Node,
    src: &[u8],
    source_module: &str,
    scope: Option<&str>,
) -> Option<ImportContext> {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())?;
    Some(context(
        &["self".to_string(), name.to_string()],
        Leaf::Module(name.to_string()),
        node,
        source_module,
        scope,
        true,
    ))
}

/// A fully-qualified expression path rooted at `crate`/`self`/`super` with at
/// least two path segments (`crate::diff::git::apply`): an import edge to the
/// prefix module, crediting the final segment. Always inline — an expression
/// executes inside some item body, so it can't form a module-level cycle.
/// Single-segment-base paths (`git::apply`) are credited via attribute access
/// instead; type-rooted paths (`Foo::new`) are not imports and return `None`.
pub fn qualified_path(
    node: Node,
    src: &[u8],
    source_module: &str,
    scope: Option<&str>,
) -> Option<ImportContext> {
    let path = node
        .child_by_field_name("path")
        .filter(|p| p.kind() == "scoped_identifier")?;
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())?;
    let mut prefix = Vec::new();
    push_path(Some(path), src, &mut prefix);
    if !matches!(
        prefix.first().map(String::as_str),
        Some("crate" | "self" | "super")
    ) {
        return None;
    }
    let mut ctx = context(
        &prefix,
        Leaf::Named {
            name: name.to_string(),
            alias: None,
        },
        node,
        source_module,
        scope,
        false,
    );
    ctx.is_inline = true;
    Some(ctx)
}

/// One flattened `use`-tree leaf.
enum Leaf {
    /// `…::name` (possibly aliased) — a symbol or submodule import.
    Named { name: String, alias: Option<String> },
    /// `…::*` — a glob import.
    Wildcard,
    /// `…::{self}` or `mod x;` — the module itself, bound as `binding`.
    Module(String),
}

/// Walk a use-tree, calling `emit(prefix_segments, leaf)` for every leaf.
fn flatten(
    node: Node,
    src: &[u8],
    prefix: &mut Vec<String>,
    emit: &mut impl FnMut(&[String], Leaf),
) {
    match node.kind() {
        "identifier" | "crate" | "super" | "self" => {
            if let Ok(text) = node.utf8_text(src) {
                if text == "self" && !prefix.is_empty() {
                    let binding = prefix.last().cloned().unwrap_or_default();
                    emit(prefix, Leaf::Module(binding));
                } else {
                    emit(
                        prefix,
                        Leaf::Named {
                            name: text.to_string(),
                            alias: None,
                        },
                    );
                }
            }
        }
        "scoped_identifier" => {
            with_path_prefix(prefix, node.child_by_field_name("path"), src, |prefix| {
                if let Some(name) = node.child_by_field_name("name") {
                    flatten(name, src, prefix, emit);
                }
            });
        }
        "use_as_clause" => {
            let alias = node
                .child_by_field_name("alias")
                .and_then(|a| a.utf8_text(src).ok())
                .map(str::to_string);
            if let Some(path) = node.child_by_field_name("path") {
                let name = leaf_text(path, src);
                with_path_prefix(prefix, path.child_by_field_name("path"), src, |prefix| {
                    emit(prefix, Leaf::Named { name, alias });
                });
            }
        }
        "scoped_use_list" => {
            with_path_prefix(prefix, node.child_by_field_name("path"), src, |prefix| {
                if let Some(list) = node.child_by_field_name("list") {
                    flatten(list, src, prefix, emit);
                }
            });
        }
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                flatten(child, src, prefix, emit);
            }
        }
        "use_wildcard" => {
            with_path_prefix(prefix, node.named_child(0), src, |prefix| {
                emit(prefix, Leaf::Wildcard);
            });
        }
        _ => {}
    }
}

fn with_path_prefix<R>(
    prefix: &mut Vec<String>,
    path: Option<Node>,
    src: &[u8],
    f: impl FnOnce(&mut Vec<String>) -> R,
) -> R {
    let len = prefix.len();
    push_path(path, src, prefix);
    let result = f(prefix);
    prefix.truncate(len);
    result
}

/// Append a (possibly scoped) path's segments to `prefix`; returns how many
/// segments were pushed so the caller can truncate back.
fn push_path(path: Option<Node>, src: &[u8], prefix: &mut Vec<String>) -> usize {
    let Some(path) = path else { return 0 };
    let before = prefix.len();
    if path.kind() == "scoped_identifier" {
        push_path(path.child_by_field_name("path"), src, prefix);
        if let Some(name) = path.child_by_field_name("name") {
            if let Ok(t) = name.utf8_text(src) {
                prefix.push(t.to_string());
            }
        }
    } else if let Ok(t) = path.utf8_text(src) {
        prefix.push(t.to_string());
    }
    prefix.len() - before
}

/// The final segment of a path node (`a::b::c` → `c`, `c` → `c`).
fn leaf_text(path: Node, src: &[u8]) -> String {
    let last = path.child_by_field_name("name").unwrap_or(path);
    last.utf8_text(src).unwrap_or_default().to_string()
}

fn context(
    prefix: &[String],
    leaf: Leaf,
    node: Node,
    source_module: &str,
    scope: Option<&str>,
    is_module_decl: bool,
) -> ImportContext {
    let (target, symbols, bindings) = match leaf {
        // Single-segment `use serde_json;` → a plain crate import.
        Leaf::Named { name, alias } if prefix.is_empty() => {
            let binding = alias.unwrap_or_else(|| name.clone());
            (name, Vec::new(), vec![binding])
        }
        Leaf::Named { name, alias } => {
            let binding = alias.clone().unwrap_or_else(|| name.clone());
            (prefix.join("::"), vec![name], vec![binding])
        }
        Leaf::Wildcard => (prefix.join("::"), vec!["*".to_string()], Vec::new()),
        Leaf::Module(binding) => (prefix.join("::"), Vec::new(), vec![binding]),
    };
    let pos = node.start_position();
    ImportContext {
        source_module: source_module.to_string(),
        target_module: target,
        imported_symbols: symbols,
        bindings,
        binding_phases: Vec::new(),
        line: pos.row + 1,
        column: pos.column + 1,
        phase: ImportPhase::Runtime,
        is_inline: scope.is_some(),
        is_module_decl,
        enclosing_scope: scope.map(str::to_string),
    }
}

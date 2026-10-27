//! Top-level declaration extraction (function/class/assignment names, `__all__`).

use crate::profiles::node_names::NodeNames;
use crate::spine::ir::Visibility;
use tree_sitter::Node;

/// Name of a `function_definition` / `class_definition`.
pub fn def_name(node: Node, src: &[u8]) -> Option<String> {
    node.field("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Dotted path of every decorator on a `decorated_definition`, call args
/// stripped. `@app.route("/")` → `app.route`; `@functools.lru_cache` →
/// `functools.lru_cache`; `@fixture` → `fixture`. The dead-code analyzer uses
/// the shape (attribute vs bare, head identifier) to classify registration vs
/// neutral vs unknown decorators.
pub fn decorator_paths(node: Node, src: &[u8]) -> Vec<String> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|c| c.kind_name() == "decorator")
        .filter_map(|c| dotted_path(c, src))
        .collect()
}

fn dotted_path(decorator: Node, src: &[u8]) -> Option<String> {
    let text = decorator.utf8_text(src).ok()?;
    let head = text.trim_start_matches('@').trim();
    let path = head.split('(').next().unwrap_or(head).trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

/// Parameter names of a `function_definition`, in order. `self`/`cls` are
/// kept (callers that want to ignore them can filter). `*args`/`**kwargs`/
/// typed/default parameters all resolve to their leading identifier.
pub fn param_names(func: Node, src: &[u8]) -> Vec<String> {
    let Some(params) = func.field("parameters") else {
        return Vec::new();
    };
    let mut cursor = params.walk();
    params
        .named_children(&mut cursor)
        .filter_map(|p| {
            if p.kind_name() == "identifier" {
                p.utf8_text(src).ok().map(str::to_string)
            } else {
                first_identifier(p, src)
            }
        })
        .collect()
}

/// Base-class names of a `class_definition` (`superclasses` argument list).
/// `class A(B, c.D)` → `["B", "c.D"]`; keyword args (`metaclass=`) are skipped.
pub fn base_classes(node: Node, src: &[u8]) -> Vec<String> {
    let Some(args) = node.field("superclasses") else {
        return Vec::new();
    };
    let mut cursor = args.walk();
    args.named_children(&mut cursor)
        .filter(|c| matches!(c.kind_name(), "identifier" | "attribute"))
        .filter_map(|c| c.utf8_text(src).ok().map(str::to_string))
        .collect()
}

fn first_identifier(node: Node, src: &[u8]) -> Option<String> {
    super::typehints::param_identifier(node)
        .and_then(|c| c.utf8_text(src).ok())
        .map(str::to_string)
}

/// Assignment target names: handles `x = ...` and `a, b = ...`.
/// `self.<attr>`.
pub fn is_self_attribute(node: Node, src: &[u8]) -> bool {
    node.kind_name() == "attribute"
        && node
            .field("object")
            .and_then(|object| object.utf8_text(src).ok())
            == Some("self")
}

pub fn assignment_targets(node: Node, src: &[u8]) -> Vec<String> {
    match node.field("left") {
        Some(left) => collect_target_names(left, src),
        None => Vec::new(),
    }
}

fn collect_target_names(node: Node, src: &[u8]) -> Vec<String> {
    match node.kind_name() {
        "identifier" => node
            .utf8_text(src)
            .ok()
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        "pattern_list" | "tuple_pattern" | "list_pattern" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .flat_map(|child| collect_target_names(child, src))
                .collect()
        }
        _ => Vec::new(),
    }
}

/// If `node` is `__all__ = [...]`, return the listed string entries.
pub fn dunder_all(node: Node, src: &[u8]) -> Option<Vec<String>> {
    let left = node.field("left")?;
    if left.utf8_text(src).ok()? != "__all__" {
        return None;
    }
    let right = node.field("right")?;
    let mut cursor = right.walk();
    let entries = right
        .named_children(&mut cursor)
        .filter(|c| c.kind_name() == "string")
        .filter_map(|c| string_value(c, src))
        .collect();
    Some(entries)
}

/// Extract the textual content of a string literal (without quotes/prefix).
pub fn string_value(node: Node, src: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    let value = node
        .named_children(&mut cursor)
        .find(|c| c.kind_name() == "string_content")
        .and_then(|c| c.utf8_text(src).ok())
        .map(str::to_string);
    value.or_else(|| (node.kind_name() == "string").then(String::new))
}

/// `_helper` is module-private; dunder and plain names are public.
pub fn visibility(name: &str) -> Visibility {
    if name.starts_with('_') && !name.starts_with("__") {
        Visibility::Private
    } else {
        Visibility::Exported
    }
}

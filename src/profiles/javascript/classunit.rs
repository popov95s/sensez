//! Per-class structural summary for the design-smell pillar (JS/TS), plus the
//! tuple-return-arity helper used by `units::analyze_function`. Split from
//! `units` to keep each file within the module size budget.

use super::symbols;
use crate::spine::ir::{ClassProperty, ClassUnit};
use tree_sitter::Node;

/// Build a [`ClassUnit`] for a `class`/`class_declaration`/`abstract_class_declaration`.
pub fn analyze_class(class: Node, src: &[u8]) -> ClassUnit {
    let mut unit = ClassUnit {
        name: symbols::def_name(class, src).unwrap_or_default(),
        start_line: class.start_position().row + 1,
        end_line: class.end_position().row + 1,
        bases: base_classes(class, src),
        is_abstract: class.kind() == "abstract_class_declaration",
        ..Default::default()
    };
    let Some(body) = class.child_by_field_name("body") else {
        return unit;
    };
    let mut cursor = body.walk();
    for member in body.named_children(&mut cursor) {
        if let Some(prop) = property_from_member(member, src) {
            unit.properties.push(prop);
            continue;
        }
        if member.kind() != "method_definition" {
            continue;
        }
        let Some(name) = symbols::def_name(member, src) else {
            continue;
        };
        unit.methods.push(name.clone());
        if let Some(mbody) = member.child_by_field_name("body") {
            if is_stub_body(mbody) {
                unit.overrides_to_stub.push(name);
            }
        }
        // `method_attr_use` is filled by the post-walk join in `traversal`.
    }
    unit
}

fn property_from_member(member: Node, src: &[u8]) -> Option<ClassProperty> {
    if member.kind().contains("method") {
        return None;
    }
    let name = member
        .child_by_field_name("name")
        .and_then(|node| node.utf8_text(src).ok())
        .map(str::trim)
        .filter(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c == '_' || c == '$' || c.is_alphanumeric())
        })?;
    let ty = type_text(member.child_by_field_name("type"), src)
        .or_else(|| instantiated_type(member.child_by_field_name("value"), src))?;
    Some(ClassProperty {
        name: name.to_string(),
        type_name: normalize_type(&ty),
        line: member.start_position().row + 1,
    })
}

fn instantiated_type(value: Option<Node>, src: &[u8]) -> Option<String> {
    let new_expr = value.filter(|node| node.kind() == "new_expression")?;
    let ctor = new_expr.child_by_field_name("constructor")?;
    let text = ctor.utf8_text(src).ok()?;
    let last = text.rsplit('.').next().unwrap_or(text);
    last.chars()
        .next()
        .is_some_and(char::is_uppercase)
        .then(|| last.to_string())
}

fn type_text(node: Option<Node>, src: &[u8]) -> Option<String> {
    let text = node.and_then(|n| n.utf8_text(src).ok())?;
    let trimmed = text.trim_start_matches(':').trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn normalize_type(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `extends Base` + TS `implements I1, I2` — every (type-)identifier named in
/// the class heritage clause.
fn base_classes(class: Node, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = class.walk();
    for child in class.children(&mut cursor) {
        if child.kind() == "class_heritage" {
            collect_idents(child, src, &mut out);
        }
    }
    out
}

fn collect_idents(node: Node, src: &[u8], out: &mut Vec<String>) {
    if matches!(node.kind(), "identifier" | "type_identifier") {
        if let Ok(t) = node.utf8_text(src) {
            out.push(t.to_string());
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_idents(child, src, out);
    }
}

/// A method body that only `throw`s (`throw new Error("not implemented")`) — the
/// JS/TS analog of a `raise NotImplementedError` stub.
fn is_stub_body(body: Node) -> bool {
    let mut cursor = body.walk();
    let stmts: Vec<Node> = body
        .named_children(&mut cursor)
        .filter(|s| s.kind() != "comment")
        .collect();
    matches!(stmts.as_slice(), [one] if one.kind() == "throw_statement")
}

/// Top-level element count of a tuple return annotation `(): [A, B, C]`, else 0.
/// JS array returns are too common to treat as tuples, so only the annotation
/// counts — the analog of Python's bare `return a, b, c`.
pub fn tuple_return_arity(func: Node, src: &[u8]) -> usize {
    let Some(text) = func
        .child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(src).ok())
    else {
        return 0;
    };
    let body = match text
        .trim_start_matches(':')
        .trim()
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
    {
        Some(b) if !b.trim().is_empty() => b,
        _ => return 0,
    };
    let mut depth = BracketDepth::default();
    let mut count = 1usize;
    for c in body.chars() {
        match c {
            '[' | '(' | '<' | '{' => depth.open(),
            ']' | ')' | '>' | '}' => depth.close(),
            ',' if depth.is_top_level() => count += 1,
            _ => {}
        }
    }
    count
}

#[derive(Default)]
struct BracketDepth {
    value: usize,
}

impl BracketDepth {
    fn open(&mut self) {
        self.value += 1;
    }

    fn close(&mut self) {
        self.value = self.value.saturating_sub(1);
    }

    fn is_top_level(&self) -> bool {
        self.value == 0
    }
}

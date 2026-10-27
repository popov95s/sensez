//! Per-class structural summary (JS/TS) and tuple return annotation helpers.

use super::symbols;
use crate::profiles::node_names::NodeNames;
use crate::profiles::typevocab::split_top_level;
use crate::profiles::walk;
use crate::spine::ir::{ClassProperty, ClassUnit};
use tree_sitter::Node;

/// Build a [`ClassUnit`] for a `class`/`class_declaration`/`abstract_class_declaration`.
pub fn analyze_class(class: Node, src: &[u8]) -> ClassUnit {
    let mut unit = ClassUnit {
        name: symbols::def_name(class, src).unwrap_or_default(),
        start_line: class.start_position().row + 1,
        end_line: class.end_position().row + 1,
        bases: base_classes(class, src),
        implements: implemented_interfaces(class, src),
        is_abstract: class.kind_name() == "abstract_class_declaration",
        ..Default::default()
    };
    let Some(body) = class.field("body") else {
        return unit;
    };
    let mut cursor = body.walk();
    for member in body.named_children(&mut cursor) {
        if let Some(prop) = property_from_member(member, src) {
            unit.properties.push(prop);
            continue;
        }
        if member.kind_name() != "method_definition" {
            continue;
        }
        let Some(name) = symbols::def_name(member, src) else {
            continue;
        };
        unit.methods.push(name.clone());
        if let Some(mbody) = member.field("body") {
            if is_stub_body(mbody) {
                unit.overrides_to_stub.push(name);
            }
        }
        // `method_attr_use` is filled by the post-walk join in `traversal`.
    }
    unit
}

fn property_from_member(member: Node, src: &[u8]) -> Option<ClassProperty> {
    if member.kind_name().contains("method") {
        return None;
    }
    let name = member
        .field("name")
        .and_then(|node| node.utf8_text(src).ok())
        .map(str::trim)
        .filter(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c == '_' || c == '$' || c.is_alphanumeric())
        })?;
    let initializer_type = instantiated_type(member.field("value"), src);
    let ty = type_text(member.field("type"), src).or_else(|| initializer_type.clone())?;
    Some(ClassProperty {
        name: name.to_string(),
        type_name: normalize_type(&ty),
        initializer_type,
        line: member.start_position().row + 1,
    })
}

fn instantiated_type(value: Option<Node>, src: &[u8]) -> Option<String> {
    let new_expr = value.filter(|node| node.kind_name() == "new_expression")?;
    let ctor = new_expr.field("constructor")?;
    let text = ctor.utf8_text(src).ok()?;
    let last = walk::last_segment(text);
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
        if child.kind_name() == "class_heritage" {
            collect_idents(child, src, &mut out);
        }
    }
    out
}

/// TS `implements I1, I2` names only.
fn implemented_interfaces(class: Node, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = class.walk();
    for heritage in class
        .children(&mut cursor)
        .filter(|child| child.kind_name() == "class_heritage")
    {
        let mut inner = heritage.walk();
        for clause in heritage
            .children(&mut inner)
            .filter(|clause| clause.kind_name() == "implements_clause")
        {
            collect_idents(clause, src, &mut out);
        }
    }
    out
}

fn collect_idents(node: Node, src: &[u8], out: &mut Vec<String>) {
    if matches!(node.kind_name(), "identifier" | "type_identifier") {
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
        .filter(|s| s.kind_name() != "comment")
        .collect();
    matches!(stmts.as_slice(), [one] if one.kind_name() == "throw_statement")
}

/// Element count of an unlabeled tuple return annotation `(): [A, B, C]`, else 0.
pub fn tuple_return_arity(elements: &[&str]) -> usize {
    if elements.iter().any(|element| is_labeled(element)) {
        return 0;
    }
    elements.len()
}

/// `(): [number, number, number]` — every element is the same primitive.
pub fn is_homogeneous_primitive_tuple(elements: &[&str]) -> bool {
    elements.first().is_some_and(|first| {
        matches!(*first, "number" | "string" | "boolean" | "bigint")
            && elements.iter().all(|element| element == first)
    })
}

/// Top-level elements of a tuple return annotation `(): [A, B]`.
pub fn tuple_return_elements<'a>(func: Node, src: &'a [u8]) -> Vec<&'a str> {
    let Some(text) = func
        .field("return_type")
        .and_then(|n| n.utf8_text(src).ok())
    else {
        return Vec::new();
    };
    match text
        .trim_start_matches(':')
        .trim()
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
    {
        Some(body) if !body.trim().is_empty() => split_top_level(body, ','),
        _ => Vec::new(),
    }
}

/// `r: number` / `r?: number` — a named tuple member.
fn is_labeled(element: &str) -> bool {
    split_top_level(element, ':').len() > 1
}

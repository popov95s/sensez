//! Casts and suppressions that silence the TypeScript checker.

use super::runtime_guard;
use crate::profiles::node_names::NodeNames;
use crate::spine::ir::{TypeEscape, TypeEscapeKind};
use tree_sitter::Node;

/// The type escape `node` writes, if any.
pub(super) fn type_escape(node: Node<'_>, src: &[u8]) -> Option<TypeEscape> {
    let (kind, label, value) = match node.kind_name() {
        "as_expression" => cast(node, src)?,
        "type_assertion" => angle_cast(node, src)?,
        "non_null_expression" => non_null_call(node, src)?,
        "comment" => suppression(node, src)?,
        _ => return None,
    };
    Some(TypeEscape {
        line: node.start_position().row + 1,
        kind,
        label: label.to_string(),
        subject: value.and_then(|value| root_identifier(value, src)),
    })
}

type Escape<'t> = (TypeEscapeKind, &'static str, Option<Node<'t>>);

fn cast<'t>(node: Node<'t>, src: &[u8]) -> Option<Escape<'t>> {
    let value = node.named_child(0)?;
    let target = node.named_child(1)?;
    if is_predefined(target, "any", src) {
        return Some((TypeEscapeKind::AnyCast, "as any", Some(value)));
    }
    let inner = unwrap_parens(value);
    let laundered = inner.kind_name() == "as_expression"
        && inner
            .named_child(1)
            .is_some_and(|ty| is_predefined(ty, "unknown", src));
    laundered.then(|| {
        (
            TypeEscapeKind::DoubleCast,
            "as unknown as",
            inner.named_child(0),
        )
    })
}

fn angle_cast<'t>(node: Node<'t>, src: &[u8]) -> Option<Escape<'t>> {
    let arguments = node.named_child(0)?;
    let any = arguments
        .named_child(0)
        .is_some_and(|ty| is_predefined(ty, "any", src));
    any.then(|| (TypeEscapeKind::AnyCast, "<any>", node.named_child(1)))
}

fn non_null_call<'t>(node: Node<'t>, src: &[u8]) -> Option<Escape<'t>> {
    let mut inner = unwrap_parens(node.named_child(0)?);
    if inner.kind_name() == "await_expression" {
        inner = unwrap_parens(inner.named_child(0)?);
    }
    let unchecked =
        inner.kind_name() == "call_expression" && !runtime_guard::is_checked(inner, src);
    unchecked.then_some((TypeEscapeKind::NonNullCall, "call()!", None))
}

fn suppression<'t>(node: Node<'t>, src: &[u8]) -> Option<Escape<'t>> {
    let body = node
        .utf8_text(src)
        .ok()?
        .trim_start_matches('/')
        .trim_start_matches('*')
        .trim_end_matches('/')
        .trim_end_matches('*')
        .trim();
    let (kind, label, rest) = if let Some(rest) = body.strip_prefix("@ts-ignore") {
        (TypeEscapeKind::BlanketSuppression, "@ts-ignore", rest)
    } else {
        let rest = body.strip_prefix("@ts-expect-error")?;
        (
            TypeEscapeKind::UnexplainedSuppression,
            "@ts-expect-error",
            rest,
        )
    };
    let reason = rest.trim_matches(|c: char| c.is_whitespace() || matches!(c, ':' | '-' | '—'));
    reason.is_empty().then_some((kind, label, None))
}

fn is_predefined(node: Node<'_>, name: &str, src: &[u8]) -> bool {
    node.kind_name() == "predefined_type" && node.utf8_text(src).is_ok_and(|text| text == name)
}

fn unwrap_parens(node: Node<'_>) -> Node<'_> {
    match node.kind_name() {
        "parenthesized_expression" => node.named_child(0).map_or(node, unwrap_parens),
        _ => node,
    }
}

fn root_identifier(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind_name() {
        "identifier" | "this" => node.utf8_text(src).ok().map(str::to_string),
        "member_expression" | "subscript_expression" => root_identifier(node.field("object")?, src),
        "parenthesized_expression" | "non_null_expression" => {
            root_identifier(node.named_child(0)?, src)
        }
        _ => None,
    }
}

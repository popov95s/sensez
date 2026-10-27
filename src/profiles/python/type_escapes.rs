//! Casts and suppressions that silence a Python type checker.

use crate::profiles::node_names::NodeNames;
use crate::spine::ir::{TypeEscape, TypeEscapeKind};
use tree_sitter::Node;

const ANY_SPELLINGS: &[&str] = &["Any", "typing.Any", "t.Any", "\"Any\"", "'Any'"];

/// The type escape `node` writes, if any.
pub(super) fn type_escape(node: Node<'_>, src: &[u8]) -> Option<TypeEscape> {
    let line = node.start_position().row + 1;
    match node.kind_name() {
        "comment" => bare_ignore(node, src).then(|| TypeEscape {
            line,
            kind: TypeEscapeKind::UnexplainedSuppression,
            label: "# type: ignore".to_string(),
            subject: None,
        }),
        "call" => any_cast(node, src).map(|subject| TypeEscape {
            line,
            kind: TypeEscapeKind::AnyCast,
            label: "cast(Any, …)".to_string(),
            subject,
        }),
        _ => None,
    }
}

/// `# type: ignore` with no `[error-code]`.
fn bare_ignore(node: Node<'_>, src: &[u8]) -> bool {
    let Ok(text) = node.utf8_text(src) else {
        return false;
    };
    let Some((_, after)) = text.split_once("type:") else {
        return false;
    };
    after
        .trim_start()
        .strip_prefix("ignore")
        .is_some_and(|rest| !rest.starts_with(|c: char| c == '[' || c.is_alphanumeric()))
}

/// `cast(Any, value)`: the root name of `value`, if the call is one.
fn any_cast(node: Node<'_>, src: &[u8]) -> Option<Option<String>> {
    let callee = node.field("function")?.utf8_text(src).ok()?;
    if callee != "cast" && !callee.ends_with(".cast") {
        return None;
    }
    let arguments = node.field("arguments")?;
    let target = arguments.named_child(0)?.utf8_text(src).ok()?;
    ANY_SPELLINGS.contains(&target).then(|| {
        arguments
            .named_child(1)
            .and_then(|value| root_name(value, src))
    })
}

fn root_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind_name() {
        "identifier" => node.utf8_text(src).ok().map(str::to_string),
        "attribute" | "subscript" => root_name(node.named_child(0)?, src),
        _ => None,
    }
}

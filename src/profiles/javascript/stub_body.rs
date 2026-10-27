//! JS/TS function bodies that only defer their work.

use crate::profiles::node_names::NodeNames;
use crate::profiles::todo_comment::has_todo_comment;
use crate::spine::ir::StubBody;
use tree_sitter::Node;

/// The stub shape of `func`'s block body, if its only statement defers the work.
pub(super) fn stub_body(func: Node, src: &[u8]) -> Option<StubBody> {
    let body = func.field("body")?;
    if body.kind_name() != "statement_block" {
        return None;
    }
    let mut cursor = body.walk();
    let statements: Vec<Node> = body
        .named_children(&mut cursor)
        .filter(|statement| statement.kind_name() != "comment")
        .collect();
    match statements.as_slice() {
        [] => todo_empty(body, src),
        [only] if only.kind_name() == "throw_statement" => thrown(*only, src),
        [only] if returns_nothing(*only) => todo_empty(body, src),
        _ => None,
    }
}

fn todo_empty(body: Node, src: &[u8]) -> Option<StubBody> {
    has_todo_comment(body, src, "comment").then_some(StubBody::TodoEmpty)
}

fn returns_nothing(statement: Node) -> bool {
    statement.kind_name() == "return_statement"
        && statement
            .named_child(0)
            .is_none_or(|value| matches!(value.kind_name(), "null" | "undefined"))
}

/// `throw new Error("msg")` / `throw Error("msg")` / `throw new NotImplementedError()`.
fn thrown(statement: Node, src: &[u8]) -> Option<StubBody> {
    let error = statement.named_child(0)?;
    let callee = error
        .field("constructor")
        .or_else(|| error.field("function"))?;
    if callee.utf8_text(src).ok()?.contains("NotImplemented") {
        return Some(StubBody::NotImplemented);
    }
    let message = error.field("arguments")?.named_child(0)?;
    matches!(message.kind_name(), "string" | "template_string")
        .then(|| message.utf8_text(src).ok())
        .flatten()
        .map(|text| StubBody::Throws(text.trim_matches(['"', '\'', '`']).to_string()))
}

//! Python function bodies that only defer their work.

use super::symbols;
use super::units::{is_abstract_base, is_docstring};
use crate::profiles::node_query::last_segment;
use crate::profiles::todo_comment::has_todo_comment;
use crate::spine::ir::StubBody;
use tree_sitter::Node;

/// The stub shape of `func`'s body; methods count only under a `TODO`/`FIXME`.
pub(super) fn stub_body(func: Node, src: &[u8], is_method: bool) -> Option<StubBody> {
    if is_declaration(func, src) {
        return None;
    }
    let body = func.child_by_field_name("body")?;
    let mut cursor = body.walk();
    let statements: Vec<Node> = body
        .named_children(&mut cursor)
        .filter(|statement| statement.kind() != "comment" && !is_docstring(*statement, src))
        .collect();
    let [only] = statements.as_slice() else {
        return None;
    };
    let marked = has_todo_comment(func, src, "comment");
    let stub = match only.kind() {
        "pass_statement" => StubBody::Placeholder,
        "expression_statement" if is_ellipsis(*only) => StubBody::Placeholder,
        "raise_statement" if raises_not_implemented(*only, src) => StubBody::NotImplemented,
        "return_statement" if marked && returns_nothing(*only) => StubBody::TodoEmpty,
        _ => return None,
    };
    (marked || !is_method).then_some(stub)
}

/// `@abstractmethod`/`@overload` signatures and members of `ABC`/`Protocol` classes.
fn is_declaration(func: Node, src: &[u8]) -> bool {
    let decorated = func
        .parent()
        .filter(|parent| parent.kind() == "decorated_definition");
    let declared = decorated.is_some_and(|node| {
        symbols::decorator_paths(node, src)
            .iter()
            .any(|path| matches!(last_segment(path), "abstractmethod" | "overload"))
    });
    let class = decorated
        .unwrap_or(func)
        .parent()
        .and_then(|block| block.parent())
        .filter(|parent| parent.kind() == "class_definition");
    declared
        || class.is_some_and(|class| {
            symbols::base_classes(class, src)
                .iter()
                .any(|base| is_abstract_base(base))
        })
}

fn is_ellipsis(statement: Node) -> bool {
    statement
        .named_child(0)
        .is_some_and(|value| value.kind() == "ellipsis")
}

fn raises_not_implemented(statement: Node, src: &[u8]) -> bool {
    statement
        .utf8_text(src)
        .is_ok_and(|text| text.contains("NotImplementedError"))
}

fn returns_nothing(statement: Node) -> bool {
    statement
        .named_child(0)
        .is_none_or(|value| value.kind() == "none")
}

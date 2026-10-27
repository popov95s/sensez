//! Python writes to `self` members and module-level names.

use super::control_shapes::is_function;
use crate::profiles::lazy_init::is_guarded_by;
use crate::profiles::lineage::Lineage;
use crate::spine::ir::{FunctionUnit, StateWrite, StateWriteKind};
use tree_sitter::Node;

/// Record the write an assignment `node` performs, when it is one.
pub fn scan(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    if !matches!(node.kind(), "assignment" | "augmented_assignment") {
        return;
    }
    let write = node
        .child_by_field_name("left")
        .and_then(|target| assignment(node, target, src));
    unit.state_writes.extend(write);
}

fn assignment(node: Node, target: Node, src: &[u8]) -> Option<StateWrite> {
    let text = target.utf8_text(src).ok()?;
    let root = chain_root(target)?.utf8_text(src).ok()?;
    let own = root == "self" && target.kind() != "identifier";
    let declared_outer = target.kind() == "identifier" && is_declared_outer(node, root, src);
    if target.kind() == "identifier" && !declared_outer {
        return None;
    }
    let key = match own {
        true => target
            .child_by_field_name("attribute")
            .and_then(|attribute| attribute.utf8_text(src).ok())
            .unwrap_or(root),
        false => root,
    };
    let kind = match (
        is_guarded_by(
            Lineage::detached(node),
            src,
            key,
            "if_statement",
            is_function,
            super::wording::WORDING.identifier_extra_chars,
        ),
        own,
    ) {
        (true, _) => StateWriteKind::LazyInit,
        (false, true) => StateWriteKind::OwnMember,
        (false, false) => StateWriteKind::OuterName,
    };
    Some(StateWrite {
        line: node.start_position().row + 1,
        target: text.to_string(),
        kind,
    })
}

fn chain_root(node: Node) -> Option<Node> {
    match node.kind() {
        "identifier" => Some(node),
        "attribute" | "subscript" => chain_root(
            node.child_by_field_name("object")
                .or_else(|| node.child_by_field_name("value"))?,
        ),
        _ => None,
    }
}

/// `name` is listed in a `global` / `nonlocal` statement of the enclosing function.
fn is_declared_outer(node: Node, name: &str, src: &[u8]) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if is_function(parent.kind()) {
            return parent
                .child_by_field_name("body")
                .is_some_and(|body| declares(body, name, src));
        }
        current = parent;
    }
    false
}

fn declares(body: Node, name: &str, src: &[u8]) -> bool {
    let mut cursor = body.walk();
    let declared = body
        .named_children(&mut cursor)
        .filter(|statement| matches!(statement.kind(), "global_statement" | "nonlocal_statement"))
        .any(|statement| {
            let mut names = statement.walk();
            let found = statement
                .named_children(&mut names)
                .any(|ident| ident.utf8_text(src).ok() == Some(name));
            found
        });
    declared
}

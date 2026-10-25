//! Shared matching of assertion sides that cannot differ.

use crate::spine::ir::EchoShape;
use tree_sitter::Node;

pub(crate) struct EchoSyntax {
    pub literal_kinds: &'static [&'static str],
    /// Kinds whose result may differ between two evaluations (calls, `new`, `await`).
    pub volatile_kinds: &'static [&'static str],
    pub member_kind: &'static str,
    pub object_field: &'static str,
    pub property_field: &'static str,
    pub block_kinds: &'static [&'static str],
    pub comment_kind: &'static str,
    pub alias: fn(Node<'_>, &[u8]) -> Option<Alias>,
}

/// A statement binding `name` to the identifier `source` unchanged.
pub(crate) struct Alias {
    pub name: String,
    pub source: String,
}

/// How `actual` and `expected` echo each other, if they can never differ.
pub(crate) fn echo_shape(
    actual: Node<'_>,
    expected: Node<'_>,
    src: &[u8],
    syntax: &EchoSyntax,
) -> Option<EchoShape> {
    if is_volatile(actual, syntax) || is_volatile(expected, syntax) {
        return None;
    }
    if tokens(actual, src, syntax) == tokens(expected, src, syntax) {
        let literal = syntax.literal_kinds.contains(&actual.kind());
        return Some(if literal {
            EchoShape::Literals
        } else {
            EchoShape::SameExpression
        });
    }
    is_pass_through(actual, expected, src, syntax).then_some(EchoShape::PassThrough)
}

fn is_volatile(node: Node<'_>, syntax: &EchoSyntax) -> bool {
    if syntax.volatile_kinds.contains(&node.kind()) {
        return true;
    }
    let mut cursor = node.walk();
    let volatile = node
        .named_children(&mut cursor)
        .any(|child| is_volatile(child, syntax));
    volatile
}

fn tokens<'a>(node: Node<'_>, src: &'a [u8], syntax: &EchoSyntax) -> Vec<&'a str> {
    if node.child_count() == 0 {
        return node.utf8_text(src).ok().into_iter().collect();
    }
    let mut cursor = node.walk();
    let leaves = node
        .children(&mut cursor)
        .filter(|child| child.kind() != syntax.comment_kind)
        .flat_map(|child| tokens(child, src, syntax))
        .collect();
    leaves
}

/// Same member path read from two roots, one bound to the other earlier in the test.
fn is_pass_through(actual: Node<'_>, expected: Node<'_>, src: &[u8], syntax: &EchoSyntax) -> bool {
    let (Some((actual_root, actual_path)), Some((expected_root, expected_path))) = (
        member_path(actual, src, syntax),
        member_path(expected, src, syntax),
    ) else {
        return false;
    };
    if actual_path.is_empty() || actual_path != expected_path || actual_root == expected_root {
        return false;
    }
    aliases_before(actual, src, syntax).iter().any(|alias| {
        (alias.name == actual_root && alias.source == expected_root)
            || (alias.name == expected_root && alias.source == actual_root)
    })
}

/// Root identifier and property names of a plain `a.b.c` chain.
fn member_path<'a>(
    node: Node<'_>,
    src: &'a [u8],
    syntax: &EchoSyntax,
) -> Option<(&'a str, Vec<&'a str>)> {
    if node.kind() == "identifier" {
        return Some((node.utf8_text(src).ok()?, Vec::new()));
    }
    if node.kind() != syntax.member_kind {
        return None;
    }
    let (root, mut path) =
        member_path(node.child_by_field_name(syntax.object_field)?, src, syntax)?;
    path.push(
        node.child_by_field_name(syntax.property_field)?
            .utf8_text(src)
            .ok()?,
    );
    Some((root, path))
}

/// Identifier aliases bound by statements preceding `node` in its enclosing blocks.
fn aliases_before(node: Node<'_>, src: &[u8], syntax: &EchoSyntax) -> Vec<Alias> {
    let mut found = Vec::new();
    let mut current = node;
    while let Some(parent) = current.parent() {
        if syntax.block_kinds.contains(&parent.kind()) {
            let mut cursor = parent.walk();
            found.extend(
                parent
                    .named_children(&mut cursor)
                    .take_while(|statement| statement.end_byte() <= current.start_byte())
                    .filter_map(|statement| (syntax.alias)(statement, src)),
            );
        }
        current = parent;
    }
    found
}

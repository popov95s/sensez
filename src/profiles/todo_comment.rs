//! `TODO`/`FIXME` markers in a function's own comments.

use crate::profiles::node_names::NodeNames;
use tree_sitter::Node;

/// Whether a `comment_kind` node under `node` carries a `TODO` or `FIXME` marker.
pub(crate) fn has_todo_comment(node: Node, src: &[u8], comment_kind: &str) -> bool {
    if node.kind_name() == comment_kind {
        return node.utf8_text(src).is_ok_and(is_todo);
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| has_todo_comment(child, src, comment_kind));
    found
}

fn is_todo(text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| matches!(word, "TODO" | "FIXME"))
}

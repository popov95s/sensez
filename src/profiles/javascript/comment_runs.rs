//! Runs of comment children on adjacent lines, classified together once the run ends.

use super::comment_syntax::COMMENT_SYNTAX;
use crate::profiles::node_names::NodeNames;
use crate::profiles::walk;
use crate::spine::ir::Walked;
use tree_sitter::Node;

/// The comment children of one node seen so far on adjacent lines.
#[derive(Default)]
pub struct CommentRun<'t> {
    comments: Vec<Node<'t>>,
}

impl<'t> CommentRun<'t> {
    /// Take `child`, the next named child of `parent`, recording the run it ends.
    pub fn push(&mut self, out: &mut Walked, child: Node<'t>, parent: Node<'t>, src: &[u8]) {
        let is_comment = child.kind_name() == "comment";
        let joins = is_comment
            && self
                .comments
                .last()
                .is_none_or(|last| last.end_position().row + 1 >= child.start_position().row);
        if !joins {
            self.flush(out, parent, src);
        }
        if is_comment {
            self.comments.push(child);
        }
    }

    /// Record the open run.
    pub fn flush(&mut self, out: &mut Walked, parent: Node<'t>, src: &[u8]) {
        walk::record_comment_block(out, &self.comments, parent, src, &COMMENT_SYNTAX);
        self.comments.clear();
    }
}

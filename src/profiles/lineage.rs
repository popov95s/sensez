//! Parent lookups along the walker's current path, without tree-sitter's descent from the root.

use tree_sitter::Node;

/// A node and its ancestors as the walker holds them; nodes off the path ask tree-sitter.
#[derive(Clone, Copy)]
pub(crate) struct Lineage<'a, 'tree> {
    ancestors: &'a [Node<'tree>],
    node: Node<'tree>,
}

impl<'a, 'tree> Lineage<'a, 'tree> {
    /// `node` under `ancestors`, outermost first.
    pub(crate) fn new(ancestors: &'a [Node<'tree>], node: Node<'tree>) -> Self {
        Self { ancestors, node }
    }

    /// `node` with no known ancestors: every lookup asks tree-sitter.
    pub(crate) fn detached(node: Node<'tree>) -> Self {
        Self::new(&[], node)
    }

    pub(crate) fn node(&self) -> Node<'tree> {
        self.node
    }

    /// Parent of `node`, read from the path when `node` is on it.
    pub(crate) fn parent(&self, node: Node<'tree>) -> Option<Node<'tree>> {
        let depth = if node.id() == self.node.id() {
            Some(self.ancestors.len())
        } else {
            self.ancestors
                .iter()
                .rposition(|step| step.id() == node.id())
        };
        match depth {
            Some(depth) if depth > 0 => Some(self.ancestors[depth - 1]),
            _ => node.parent(),
        }
    }

    /// Ancestors of `node`, nearest first.
    pub(crate) fn ancestors(
        self,
        node: Node<'tree>,
    ) -> impl Iterator<Item = Node<'tree>> + Clone + 'a {
        std::iter::successors(self.parent(node), move |step| self.parent(*step))
    }
}

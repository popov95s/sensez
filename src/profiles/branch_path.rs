//! Mutually exclusive branch arms enclosing a node.

use crate::spine::ir::BranchPath;
use tree_sitter::Node;

/// Per-language conditional shapes whose arms never run together.
pub(crate) struct BranchArms {
    /// Conditional kind and the fields holding its arms (`if` consequence/alternative).
    pub(crate) fields: &'static [(&'static str, &'static [&'static str])],
    /// Kinds whose every named child is its own arm (`switch` body, `match` block).
    pub(crate) children: &'static [&'static str],
}

/// `(conditional id, arm index)` for every enclosing arm, outermost first.
pub(crate) fn branch_path(node: Node, arms: &BranchArms) -> BranchPath {
    let mut path = Vec::new();
    let mut child = node;
    while let Some(parent) = child.parent() {
        if let Some(arm) = arm_index(parent, child, arms) {
            path.push((parent.id(), arm));
        }
        child = parent;
    }
    path.reverse();
    path
}

fn arm_index(parent: Node, child: Node, arms: &BranchArms) -> Option<usize> {
    let mut cursor = parent.walk();
    if arms.children.contains(&parent.kind()) {
        return parent
            .named_children(&mut cursor)
            .position(|arm| arm.id() == child.id());
    }
    let (_, fields) = arms
        .fields
        .iter()
        .find(|(kind, _)| *kind == parent.kind())?;
    let arm_ids: Vec<usize> = fields
        .iter()
        .flat_map(|field| {
            parent
                .children_by_field_name(field, &mut cursor)
                .map(|arm| arm.id())
                .collect::<Vec<_>>()
        })
        .collect();
    arm_ids.iter().position(|id| *id == child.id())
}

//! Language-neutral structural fingerprints for repeated guard detection.

use crate::spine::ir::FunctionUnit;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tree_sitter::Node;

pub(crate) fn record_repeated_guard(
    unit: &mut FunctionUnit,
    guards: &mut HashMap<u64, usize>,
    condition: Node,
    ancestor: Option<Node>,
    line: usize,
    src: &[u8],
) {
    let context = ancestor.map(|node| fingerprint(node, src)).unwrap_or(0);
    let fingerprint = combine(context, fingerprint(condition, src));
    if guards
        .insert(fingerprint, line)
        .is_some_and(|previous| line.saturating_sub(previous) <= 5)
    {
        unit.review_risks.repeated_guards += 1;
    }
}

fn fingerprint(node: Node<'_>, src: &[u8]) -> u64 {
    fn hash_node(node: Node<'_>, src: &[u8], state: &mut DefaultHasher) {
        node.kind().hash(state);
        if node.child_count() == 0 {
            node.utf8_text(src).unwrap_or_default().hash(state);
            return;
        }
        for index in 0..node.child_count() {
            if let Some(child) = node.child(index) {
                hash_node(child, src, state);
            }
        }
    }

    let mut state = DefaultHasher::new();
    hash_node(node, src, &mut state);
    state.finish()
}

fn combine(ancestor: u64, condition: u64) -> u64 {
    let mut state = DefaultHasher::new();
    ancestor.hash(&mut state);
    condition.hash(&mut state);
    state.finish()
}

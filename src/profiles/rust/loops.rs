//! Rust loop frames: loop kind, bindings and bound.

use crate::profiles::for_loop::{self, ForLoopShape, RangeEnd};
use crate::spine::ir::LoopFrame;
use tree_sitter::Node;

const FOR_LOOP: ForLoopShape = ForLoopShape {
    kind: "for_expression",
    binding_field: "pattern",
    subject_field: "value",
    trip_count,
    is_bounded: super::performance::is_bounded_loop,
};

pub(super) fn loop_frame(node: Node, src: &[u8], parent: Option<usize>) -> LoopFrame {
    for_loop::loop_frame(node, src, parent, &FOR_LOOP)
}

/// Trip count of `xs.take(N)`, `A..B` or `A..=B` with literal bounds.
fn trip_count(compact: &str) -> Option<usize> {
    if let Some(tail) = compact.split(".take(").nth(1) {
        return tail.split(')').next()?.parse().ok();
    }
    let range = compact.trim_start_matches('(').trim_end_matches(')');
    let (start, end) = range.split_once("..")?;
    let (end, range_end) = match end.strip_prefix('=') {
        Some(end) => (end, RangeEnd::Inclusive),
        None => (end, RangeEnd::Exclusive),
    };
    let start = if start.is_empty() {
        0
    } else {
        start.parse().ok()?
    };
    Some(for_loop::trip_count(start, end.parse().ok()?, range_end))
}

#[cfg(test)]
#[path = "loops_tests.rs"]
mod tests;

//! Python loop frames: loop kind, bindings and bound.

use crate::profiles::for_loop::{self, ForLoopShape, RangeEnd};
use crate::spine::ir::LoopFrame;
use tree_sitter::Node;

const FOR_LOOP: ForLoopShape = ForLoopShape {
    kind: "for_statement",
    binding_field: "left",
    subject_field: "right",
    trip_count,
    is_bounded: super::performance::is_bounded_loop,
};

pub fn loop_frame(node: Node, src: &[u8], parent: Option<usize>) -> LoopFrame {
    for_loop::loop_frame(node, src, parent, &FOR_LOOP)
}

/// Trip count of `range(stop)`, `range(start, stop)` or `range(start, stop, step)` with literal arguments.
fn trip_count(compact: &str) -> Option<usize> {
    let args = compact.strip_prefix("range(")?.strip_suffix(')')?;
    let args: Vec<usize> = args
        .split(',')
        .map(|arg| arg.parse().ok())
        .collect::<Option<_>>()?;
    match args[..] {
        [stop] => Some(stop),
        [start, stop] => Some(for_loop::trip_count(start, stop, RangeEnd::Exclusive)),
        [start, stop, step] if step > 0 => {
            Some(for_loop::trip_count(start, stop, RangeEnd::Exclusive).div_ceil(step))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::trip_count;

    #[test]
    fn stop_only_range_counts_stop() {
        assert_eq!(trip_count("range(3)"), Some(3));
    }

    #[test]
    fn start_stop_range_subtracts_start() {
        assert_eq!(trip_count("range(1000,1003)"), Some(3));
    }

    #[test]
    fn stepped_range_divides_by_step() {
        assert_eq!(trip_count("range(0,10,3)"), Some(4));
    }

    #[test]
    fn non_literal_range_has_no_count() {
        assert_eq!(trip_count("range(n)"), None);
    }
}

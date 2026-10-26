//! Per-hit line anchors for findings that aggregate several offending lines.

use std::collections::BTreeSet;

/// Sorted, de-duplicated hit lines.
pub(super) fn hit_lines(lines: impl Iterator<Item = usize>) -> Vec<usize> {
    lines.collect::<BTreeSet<_>>().into_iter().collect()
}

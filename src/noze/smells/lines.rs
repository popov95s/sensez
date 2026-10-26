//! Per-hit line anchors for findings that aggregate several offending lines.

use std::collections::BTreeSet;

/// Lines shown in a message before the rest collapse into "+N more".
const SHOWN_LINES: usize = 3;

/// Sorted, de-duplicated hit lines.
pub(super) fn hit_lines(lines: impl Iterator<Item = usize>) -> Vec<usize> {
    lines.collect::<BTreeSet<_>>().into_iter().collect()
}

/// `L12, L40, L51 (+2 more)`.
pub(super) fn line_list(lines: &[usize]) -> String {
    let shown: Vec<String> = lines
        .iter()
        .take(SHOWN_LINES)
        .map(|line| format!("L{line}"))
        .collect();
    let rest = lines.len().saturating_sub(SHOWN_LINES);
    if rest == 0 {
        shown.join(", ")
    } else {
        format!("{} (+{rest} more)", shown.join(", "))
    }
}

//! Fold clone classes that only restate an overlapping, wider class.

use crate::report::{AbsorbedClone, CloneClass, CloneOccurrence};
use std::cmp::Reverse;

const DEFAULT_CLONE_HINT: &str = "structural clone";

/// Fold a class into a kept one when each of its occurrences overlaps a distinct kept occurrence.
pub(super) fn consolidate(mut classes: Vec<CloneClass>) -> Vec<CloneClass> {
    classes.sort_by_key(|c| {
        (
            Reverse(c.occurrences.len()),
            c.detector,
            Reverse(c.token_length),
        )
    });
    let mut kept: Vec<CloneClass> = Vec::new();
    for class in classes {
        let host = kept
            .iter_mut()
            .find_map(|host| cover(host, &class).map(|slots| (host, slots)));
        match host {
            Some((host, slots)) => absorb(host, class, &slots),
            None => kept.push(class),
        }
    }
    kept
}

/// Host occurrence index overlapped by each of `class`'s occurrences, if all are covered.
fn cover(host: &CloneClass, class: &CloneClass) -> Option<Vec<usize>> {
    let mut slots: Vec<usize> = Vec::with_capacity(class.occurrences.len());
    for occ in &class.occurrences {
        let slot = (0..host.occurrences.len())
            .find(|idx| !slots.contains(idx) && overlaps(&host.occurrences[*idx], occ))?;
        slots.push(slot);
    }
    Some(slots)
}

fn overlaps(a: &CloneOccurrence, b: &CloneOccurrence) -> bool {
    a.file == b.file && a.start_row <= b.end_row && b.start_row <= a.end_row
}

fn absorb(host: &mut CloneClass, class: CloneClass, slots: &[usize]) {
    for (occ, &slot) in class.occurrences.iter().zip(slots) {
        let target = &mut host.occurrences[slot];
        target.start_row = target.start_row.min(occ.start_row);
        target.end_row = target.end_row.max(occ.end_row);
    }
    let base = host.hint.as_deref().unwrap_or(DEFAULT_CLONE_HINT);
    let label = class.hint.as_deref().unwrap_or(DEFAULT_CLONE_HINT);
    host.hint = Some(format!(
        "{base}; also {label} ({} tokens, {} of {} copies)",
        class.token_length,
        slots.len(),
        host.occurrences.len()
    ));
    host.absorbed.push(AbsorbedClone {
        detector: class.detector,
        token_length: class.token_length,
        copies: slots.len(),
    });
}

#[cfg(test)]
#[path = "consolidate_tests.rs"]
mod tests;

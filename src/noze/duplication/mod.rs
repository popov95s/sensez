//! Pillar 1: copy-paste duplication via a generalized suffix array.
//!
//! Each file is reduced to a stream of **lexeme codes** (see `parser::lexeme`):
//! real names/methods/types/literals/operators are kept, and only
//! function-local variables collapse to one code. So clones are runs of code
//! that's genuinely the same (same calls, same constants), tolerant of local
//! variable renames — Type-2 with minimal normalization. Files are flattened
//! into one master buffer ([`flatten`]); a suffix array + LCP yield n-way
//! groups ([`clones`]), optionally gap-stitched ([`gapped`]); occurrences are
//! mapped back to file/line ranges, de-overlapped, de-duped, and ranked.

mod class_shapes;
mod clones;
mod flatten;
mod gapped;
mod near_miss;
#[cfg(test)]
mod test_support;

use crate::config::model::Duplication;
use crate::globs::build_globset;
use crate::noze::{ActionLevel, CloneClass, CloneOccurrence};
use crate::spine::parser::tokens::TokenSpan;
use crate::spine::parser::ParsedFile;
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;

/// Detect duplication per the config (path excludes, threshold, gap stitching).
///
/// Files are partitioned by language before flattening: the master token buffer
/// uses a shared code alphabet, so mixing languages would let a Python and a JS
/// function of the same control-flow shape match as a "clone". Detecting within
/// each language cohort keeps clones meaningful and lets each partition map its
/// occurrences back through its own file slice.
pub fn detect(files: &[ParsedFile], config: &Duplication) -> Vec<CloneClass> {
    if files.is_empty() || config.threshold == 0 {
        return Vec::new();
    }
    // Exclude tests/migrations from the corpus (kept in the graph elsewhere).
    let excluded = build_globset(&config.exclude);
    let kept: Vec<&ParsedFile> = files
        .iter()
        .filter(|f| !excluded.is_match(&f.path))
        .collect();

    let mut by_language: BTreeMap<crate::profiles::Language, Vec<&ParsedFile>> = BTreeMap::new();
    for file in &kept {
        by_language.entry(file.language).or_default().push(*file);
    }

    let mut seen: FxHashSet<u64> = FxHashSet::default();
    let mut out: Vec<CloneClass> = Vec::new();
    for partition in by_language.values() {
        detect_partition(partition, config, &mut seen, &mut out);
    }
    out.extend(class_shapes::detect(
        &kept,
        config.class_property_overlap_min,
    ));
    // Rank by impact: long clones with many occurrences first.
    out.sort_by_key(|c| {
        (
            action_rank(c.action),
            std::cmp::Reverse(c.token_length * c.occurrences.len()),
        )
    });
    out
}

fn action_rank(level: ActionLevel) -> u8 {
    match level {
        ActionLevel::MustFix => 0,
        ActionLevel::Warning => 1,
        ActionLevel::Advisory => 2,
        ActionLevel::Info => 3,
    }
}

/// Run the suffix-array clone pipeline over one single-language file cohort,
/// appending de-duplicated clone classes to `out`.
fn detect_partition(
    kept: &[&ParsedFile],
    config: &Duplication,
    seen: &mut FxHashSet<u64>,
    out: &mut Vec<CloneClass>,
) {
    let master = flatten::build(kept);
    let groups = clones::extract(&master, config.threshold);
    let clusters = gapped::stitch(groups, &master.spans, config.max_gap);

    for cluster in clusters {
        let occ: Vec<CloneOccurrence> = cluster
            .occ
            .iter()
            .filter_map(|&(start, end)| occurrence(kept, &master.spans, start, end))
            .collect();
        let occ = suppress_overlaps(occ);
        if occ.len() < 2 {
            continue; // a clone needs >= 2 distinct, non-overlapping locations
        }
        if !seen.insert(dedup_key(&occ, cluster.matched)) {
            continue;
        }
        out.push(CloneClass {
            action: ActionLevel::Advisory,
            token_length: cluster.matched,
            occurrences: occ,
            hint: None,
        });
    }

    // Opt-in: consistent-rename near-miss clones (function granularity). Routed
    // through the same dedup so a near-miss already covered as an exact clone
    // doesn't double-report.
    if config.near_miss {
        for class in near_miss::detect(kept, config.threshold) {
            let occ = suppress_overlaps(class.occurrences);
            if occ.len() < 2 {
                continue;
            }
            if !seen.insert(dedup_key(&occ, class.token_length)) {
                continue;
            }
            out.push(CloneClass {
                action: ActionLevel::Advisory,
                occurrences: occ,
                ..class
            });
        }
    }
}

/// Map a master-buffer token range `[start, end)` to a concrete file/line span.
fn occurrence(
    files: &[&ParsedFile],
    spans: &[TokenSpan],
    start: usize,
    end: usize,
) -> Option<CloneOccurrence> {
    let first = spans.get(start)?;
    let last = spans.get(end.saturating_sub(1))?;
    if first.file_id == flatten::NON_TOKEN || last.file_id == flatten::NON_TOKEN {
        return None; // defensive: matches never span separators
    }
    let file = files.get(first.file_id as usize)?.path.clone();
    Some(CloneOccurrence {
        file,
        start_row: first.start_row,
        end_row: last.end_row,
    })
}

/// Drop occurrences that overlap an already-kept one in the same file.
///
/// Sorted by `(file, start_row)`, kept occurrences are disjoint and increasing,
/// so the only one a candidate can overlap is the most recently kept — making
/// this O(k) rather than O(k²) (which blew up on a pattern repeated thousands
/// of times).
fn suppress_overlaps(mut occ: Vec<CloneOccurrence>) -> Vec<CloneOccurrence> {
    occ.sort_by(|a, b| (a.file.as_path(), a.start_row).cmp(&(b.file.as_path(), b.start_row)));
    let mut kept: Vec<CloneOccurrence> = Vec::new();
    for o in occ {
        let overlaps = kept
            .last()
            .is_some_and(|k| k.file == o.file && o.start_row <= k.end_row);
        if !overlaps {
            kept.push(o);
        }
    }
    kept
}

/// Order-independent key so the same clone class is reported once.
///
/// Folds each occurrence's `(file, start_row)` into a `u64` and combines them
/// commutatively (XOR of per-occurrence hashes) so location order doesn't
/// matter — no sorting, no `String` allocation in this per-class hot loop
/// (which previously blew up on patterns repeated thousands of times). `len` is
/// mixed in so clones differing only in matched length stay distinct.
fn dedup_key(occ: &[CloneOccurrence], len: usize) -> u64 {
    use std::hash::{Hash, Hasher};
    let combined = occ.iter().fold(0u64, |acc, o| {
        let mut h = rustc_hash::FxHasher::default();
        (o.file.as_path(), o.start_row).hash(&mut h);
        acc ^ h.finish()
    });
    combined.rotate_left(1) ^ (len as u64)
}

#[cfg(test)]
mod tests;

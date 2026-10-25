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
mod consolidate;
mod flatten;
#[cfg(test)]
mod gap_tests;
mod gapped;
mod lexeme_similarity;
mod near_miss;
mod open_tail;
#[cfg(test)]
mod range_tests;
#[cfg(feature = "eyez")]
mod semantic;
#[cfg(test)]
pub(crate) mod test_support;

pub(crate) use lexeme_similarity::{function_lexemes, length_ceiling_percent, similarity_percent};

use crate::config::model::Duplication;
use crate::noze::source_scope::SourceScope;
use crate::report::{ActionLevel, CloneClass, CloneDetector, CloneOccurrence, ScanIssue};
use crate::spine::parser::tokens::TokenSpan;
use crate::spine::parser::{FunctionUnit, ParsedFile};
use rayon::prelude::*;
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn top_level_functions(file: &ParsedFile) -> Vec<&FunctionUnit> {
    file.walked
        .units
        .functions
        .iter()
        .filter(|func| !func.is_nested)
        .collect()
}

pub(super) fn span_index_range(
    file: &ParsedFile,
    start_line: usize,
    end_line: usize,
) -> std::ops::Range<usize> {
    let spans = &file.walked.syntax.spans;
    let row = |span: &TokenSpan| span.start_row as usize;
    let lo = spans.partition_point(|span| row(span) < start_line);
    let hi = spans.partition_point(|span| row(span) <= end_line);
    lo..hi.max(lo)
}

#[cfg(test)]
pub fn detect(files: &[ParsedFile], config: &Duplication) -> Vec<CloneClass> {
    detect_with_root(files, config, &SourceScope::for_duplication(config), None).0
}

/// Detect duplication per the config (path excludes, threshold, gap stitching).
///
/// Files are partitioned by the profile's clone family before flattening: the
/// master token buffer uses a shared code alphabet, so mixing languages would
/// let a Python and a JS function of the same control-flow shape match as a
/// "clone". Detecting within each family keeps clones meaningful and lets each
/// partition map its occurrences back through its own file slice.
pub fn detect_with_root(
    files: &[ParsedFile],
    config: &Duplication,
    scope: &SourceScope,
    root: Option<&Path>,
) -> (Vec<CloneClass>, Vec<ScanIssue>) {
    if files.is_empty() || config.threshold == 0 {
        return (Vec::new(), Vec::new());
    }
    let kept: Vec<&ParsedFile> = files.iter().filter(|f| scope.includes(f)).collect();

    let mut by_family: BTreeMap<crate::spine::ir::Language, Vec<&ParsedFile>> = BTreeMap::new();
    for file in &kept {
        by_family
            .entry(crate::profiles::registry::clone_family(file.language))
            .or_default()
            .push(*file);
    }

    let partitions: Vec<_> = by_family.into_values().collect();
    let ((mut out, mut issues), class_shapes) = rayon::join(
        || {
            partitions
                .into_par_iter()
                .map(|partition| detect_partition(&partition, config, root))
                .reduce(
                    || (Vec::new(), Vec::new()),
                    |(mut classes, mut issues), (more_classes, more_issues)| {
                        classes.extend(more_classes);
                        issues.extend(more_issues);
                        (classes, issues)
                    },
                )
        },
        || {
            class_shapes::detect(
                &kept,
                config.class_name_duplicates,
                config.class_property_overlap_min,
            )
        },
    );
    out.extend(class_shapes);
    // Rank by impact: long clones with many occurrences first.
    out.sort_by_key(|c| {
        (
            c.action,
            std::cmp::Reverse(c.token_length * c.occurrences.len()),
        )
    });
    issues.dedup_by(|a, b| a.message == b.message);
    (out, issues)
}

/// Run the suffix-array clone pipeline over one clone-family file cohort.
fn detect_partition(
    kept: &[&ParsedFile],
    config: &Duplication,
    _root: Option<&Path>,
) -> (Vec<CloneClass>, Vec<ScanIssue>) {
    let mut seen: FxHashSet<u64> = FxHashSet::default();
    let mut out: Vec<CloneClass> = Vec::new();
    let mut master = flatten::build(kept);
    let (seed, nesting) = gapped::seed(config.threshold, config.max_gap);
    let groups = clones::extract(&mut master, seed, nesting);
    let clusters = gapped::stitch(groups, &master.spans, config.max_gap);

    for cluster in clusters
        .into_iter()
        .map(|cluster| open_tail::close_tails(cluster, &master.spans))
        .filter(|cluster| cluster.matched >= config.threshold)
    {
        let cluster_occurrences: Vec<CloneOccurrence> = cluster
            .occ
            .iter()
            .filter_map(|&(start, end)| occurrence(kept, &master.spans, start, end))
            .collect();
        let cluster_occurrences = suppress_overlaps(cluster_occurrences);
        if cluster_occurrences.len() < 2 {
            continue; // a clone needs >= 2 distinct, non-overlapping locations
        }
        if !seen.insert(dedup_key(&cluster_occurrences, cluster.matched)) {
            continue;
        }
        out.push(CloneClass {
            action: ActionLevel::Advisory,
            token_length: cluster.matched,
            occurrences: cluster_occurrences,
            hint: None,
            detector: CloneDetector::Structural,
            absorbed: Vec::new(),
        });
    }

    for class in near_miss::detect(kept, near_miss::min_tokens(config)) {
        let near_miss_occurrences = suppress_overlaps(class.occurrences);
        if near_miss_occurrences.len() < 2 {
            continue;
        }
        if !seen.insert(dedup_key(&near_miss_occurrences, class.token_length)) {
            continue;
        }
        out.push(CloneClass {
            action: ActionLevel::Advisory,
            occurrences: near_miss_occurrences,
            ..class
        });
    }

    #[cfg(feature = "eyez")]
    let issues = semantic_pass(kept, config, _root, &mut seen, &mut out);
    #[cfg(not(feature = "eyez"))]
    let issues = Vec::new();

    (consolidate::consolidate(out), issues)
}

/// Opt-in comment-backed semantic clones; an unusable embedder is a scan issue.
#[cfg(feature = "eyez")]
fn semantic_pass(
    kept: &[&ParsedFile],
    config: &Duplication,
    root: Option<&Path>,
    seen: &mut FxHashSet<u64>,
    out: &mut Vec<CloneClass>,
) -> Vec<ScanIssue> {
    let (classes, issues) = match semantic::detect(kept, &config.semantic, config.threshold, root) {
        Ok(found) => found,
        Err(err) => {
            return vec![ScanIssue {
                stage: crate::report::ScanStage::Analyze,
                file: None,
                message: format!(
                    "duplication.semantic skipped: {err:#}; set [duplication.semantic] \
                     enabled = false to run without it"
                ),
            }]
        }
    };
    for class in classes {
        let semantic_occurrences = suppress_overlaps(class.occurrences);
        if semantic_occurrences.len() < 2 {
            continue;
        }
        if !seen.insert(dedup_key(&semantic_occurrences, class.token_length)) {
            continue;
        }
        out.push(CloneClass {
            occurrences: semantic_occurrences,
            ..class
        });
    }
    issues
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
        start_row: first.start_row as usize,
        end_row: last_closed_row(spans, start, end).unwrap_or(last.start_row) as usize,
    })
}

fn last_closed_row(spans: &[TokenSpan], start: usize, end: usize) -> Option<u32> {
    (start..end)
        .filter(|&t| closes_within(spans, start, end, t))
        .map(|t| spans[t].end_row)
        .max()
}

/// Whether token `t`'s node ends before the token after match `[start, end)` in the same file.
fn closes_within(spans: &[TokenSpan], start: usize, end: usize, t: usize) -> bool {
    match spans.get(end) {
        Some(next) if next.file_id == spans[start].file_id => spans[t].end_byte <= next.start_byte,
        _ => true,
    }
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

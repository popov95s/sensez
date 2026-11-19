//! Type-3 (gapped) clone stitching.
//!
//! Exact clones separated by a small, *consistent* gap (a few inserted/edited
//! tokens) are merged into one near-duplicate. Two clusters stitch when every
//! occurrence of the first is followed — in the same file, within `max_gap`
//! tokens — by an occurrence of the second. The merged span includes the gap,
//! but `matched` counts only the genuinely-identical tokens.

use super::clones::{Nesting, RawGroup};
use crate::spine::parser::tokens::TokenSpan;
use rayon::prelude::*;
use rustc_hash::FxHashMap as HashMap;

/// A clone cluster: `matched` identical tokens across `occ` source ranges.
pub struct Cluster {
    pub matched: usize,
    /// `(start_token, end_token_exclusive)` per occurrence in the master buffer.
    pub occ: Vec<(usize, usize)>,
}

/// Each merge pass combines clusters *pairwise* (a cluster merges at most
/// once per pass), so a chain of k gap-separated fragments halves each pass
/// and fully collapses in ceil(log2(k)) passes. 8 passes therefore handle
/// chains up to 2^8 = 256 fragments — far beyond real copy-paste — while
/// bounding worst-case work on adversarial input. Real inputs exit after a
/// pass merges nothing (usually pass 1 or 2).
const MAX_MERGE_PASSES: usize = 8;

/// Minimum share of matched tokens in every stitched occurrence's span.
const MIN_DENSITY: f64 = 0.6;

/// Shortest fragment worth stitching; stops tiny idioms from chaining.
const MIN_SEED: usize = 12;

/// Fragments as short as a third of the threshold seed stitching; the stitched total still needs the threshold.
const SEED_DIVISOR: usize = 3;

/// Extraction length and nesting: stitching needs sub-threshold fragments.
pub fn seed(threshold: usize, max_gap: usize) -> (usize, Nesting) {
    match max_gap {
        0 => (threshold, Nesting::Outermost),
        _ => (
            (threshold / SEED_DIVISOR).max(MIN_SEED).min(threshold),
            Nesting::Seeded {
                report_len: threshold,
            },
        ),
    }
}

/// Stitch exact clone groups into (possibly gapped) clusters.
pub fn stitch(groups: Vec<RawGroup>, spans: &[TokenSpan], max_gap: usize) -> Vec<Cluster> {
    let mut clusters: Vec<Cluster> = groups
        .into_iter()
        .map(|g| {
            let mut occ: Vec<(usize, usize)> =
                g.positions.iter().map(|&p| (p, p + g.len)).collect();
            occ.sort_unstable();
            Cluster {
                matched: g.len,
                occ,
            }
        })
        .collect();
    if max_gap == 0 {
        return clusters;
    }
    // Bounded fixpoint: each pass merges independent adjacent pairs; chains
    // (X gap Y gap Z) collapse over successive passes (see MAX_MERGE_PASSES).
    for _ in 0..MAX_MERGE_PASSES {
        if !merge_pass(&mut clusters, spans, max_gap) {
            break;
        }
    }
    clusters
}

/// Occurrence starts: start token -> `(cluster, end)` of every occurrence there.
type Starts = HashMap<usize, Vec<(usize, usize)>>;

fn merge_pass(clusters: &mut Vec<Cluster>, spans: &[TokenSpan], max_gap: usize) -> bool {
    let mut starts: Starts = HashMap::default();
    for (ci, c) in clusters.iter().enumerate() {
        for &(s, e) in &c.occ {
            starts.entry(s).or_default().push((ci, e));
        }
    }
    // Candidates do not depend on `consumed`, so they are found in parallel and applied in order.
    let found: Vec<Option<(usize, Cluster)>> = (0..clusters.len())
        .into_par_iter()
        .map(|i| stitched(clusters, i, &starts, spans, max_gap))
        .collect();
    let mut consumed = vec![false; clusters.len()];
    let mut merged: Vec<Cluster> = Vec::new();
    for (i, found) in found.into_iter().enumerate() {
        if consumed[i] {
            continue;
        }
        if let Some((j, candidate)) = found {
            if !consumed[j] && is_dense(&candidate) {
                merged.push(candidate);
                consumed[i] = true;
                consumed[j] = true;
            }
        }
    }
    if merged.is_empty() {
        return false;
    }
    let kept: Vec<Cluster> = std::mem::take(clusters)
        .into_iter()
        .enumerate()
        .filter_map(|(i, c)| (!consumed[i]).then_some(c))
        .collect();
    *clusters = kept;
    clusters.extend(merged);
    true
}

/// Cluster `i` stitched to the one same-sized cluster following (or overlapping) every occurrence within the gap.
fn stitched(
    clusters: &[Cluster],
    i: usize,
    starts: &Starts,
    spans: &[TokenSpan],
    max_gap: usize,
) -> Option<(usize, Cluster)> {
    let c = &clusters[i];
    let mut follower: Option<usize> = None;
    let mut overlap = 0;
    let mut occ = Vec::with_capacity(c.occ.len());
    for &(s, e) in &c.occ {
        let lo = (s + 1).max(e.saturating_sub(max_gap));
        let (j, start, end) = (e..=e + max_gap)
            .chain((lo..e).rev())
            .find_map(|cand| follower_at(clusters, i, starts, spans, (s, e), cand))?;
        if *follower.get_or_insert(j) != j {
            return None;
        }
        overlap = overlap.max(e.saturating_sub(start));
        occ.push((s, end));
    }
    let j = follower?;
    occ.sort_unstable();
    let matched = (c.matched + clusters[j].matched).saturating_sub(overlap);
    Some((j, Cluster { matched, occ }))
}

/// A same-sized cluster other than `i` with an occurrence starting at `cand` that ends past occurrence `(s, e)`.
fn follower_at(
    clusters: &[Cluster],
    i: usize,
    starts: &Starts,
    spans: &[TokenSpan],
    (s, e): (usize, usize),
    cand: usize,
) -> Option<(usize, usize, usize)> {
    starts
        .get(&cand)?
        .iter()
        .find(|&&(j, end)| {
            j != i
                && end > e
                && clusters[j].occ.len() == clusters[i].occ.len()
                && same_file(spans, s, cand)
        })
        .map(|&(j, end)| (j, cand, end))
}

fn is_dense(c: &Cluster) -> bool {
    c.occ
        .iter()
        .all(|&(s, e)| c.matched as f64 >= (e - s) as f64 * MIN_DENSITY)
}

fn same_file(spans: &[TokenSpan], a: usize, b: usize) -> bool {
    match (spans.get(a), spans.get(b)) {
        (Some(x), Some(y)) => x.file_id == y.file_id,
        _ => false,
    }
}

#[cfg(test)]
#[path = "gapped_order_tests.rs"]
mod order_tests;

#[cfg(test)]
#[path = "gapped_tests.rs"]
mod tests;

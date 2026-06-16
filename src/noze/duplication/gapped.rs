//! Type-3 (gapped) clone stitching.
//!
//! Exact clones separated by a small, *consistent* gap (a few inserted/edited
//! tokens) are merged into one near-duplicate. Two clusters stitch when every
//! occurrence of the first is followed — in the same file, within `max_gap`
//! tokens — by an occurrence of the second. The merged span includes the gap,
//! but `matched` counts only the genuinely-identical tokens.

use super::clones::RawGroup;
use crate::spine::parser::tokens::TokenSpan;
use std::collections::HashMap;

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

fn merge_pass(clusters: &mut Vec<Cluster>, spans: &[TokenSpan], max_gap: usize) -> bool {
    let mut start_of: HashMap<usize, usize> = HashMap::new();
    for (ci, c) in clusters.iter().enumerate() {
        for &(s, _) in &c.occ {
            start_of.insert(s, ci);
        }
    }
    let mut consumed = vec![false; clusters.len()];
    let mut merged: Vec<Cluster> = Vec::new();
    for i in 0..clusters.len() {
        if consumed[i] {
            continue;
        }
        if let Some(j) = unique_follower(&clusters[i], &start_of, spans, max_gap) {
            if j != i && !consumed[j] && clusters[i].occ.len() == clusters[j].occ.len() {
                merged.push(merge_two(&clusters[i], &clusters[j], spans, max_gap));
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

/// The single cluster that follows *every* occurrence of `c` within the gap.
fn unique_follower(
    c: &Cluster,
    start_of: &HashMap<usize, usize>,
    spans: &[TokenSpan],
    max_gap: usize,
) -> Option<usize> {
    let mut follower: Option<usize> = None;
    for &(s, e) in &c.occ {
        let hit = (e..=e + max_gap).find_map(|cand| {
            start_of
                .get(&cand)
                .filter(|_| same_file(spans, s, cand))
                .copied()
        })?;
        match follower {
            None => follower = Some(hit),
            Some(f) if f != hit => return None,
            _ => {}
        }
    }
    follower
}

fn merge_two(a: &Cluster, b: &Cluster, spans: &[TokenSpan], max_gap: usize) -> Cluster {
    let mut occ: Vec<(usize, usize)> = a
        .occ
        .iter()
        .filter_map(|&(s, e)| {
            b.occ
                .iter()
                .find(|&&(bs, _)| bs >= e && bs <= e + max_gap && same_file(spans, s, bs))
                .map(|&(_, be)| (s, be))
        })
        .collect();
    occ.sort_unstable();
    Cluster {
        matched: a.matched + b.matched,
        occ,
    }
}

fn same_file(spans: &[TokenSpan], a: usize, b: usize) -> bool {
    match (spans.get(a), spans.get(b)) {
        (Some(x), Some(y)) => x.file_id == y.file_id,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Layout: indices 0..=5 file 0, 6 separator, 7..=12 file 1.
    fn spans() -> Vec<TokenSpan> {
        let f = |id: u32| TokenSpan {
            file_id: id,
            start_row: 1,
            end_row: 1,
        };
        let mut s: Vec<TokenSpan> = (0..6).map(|_| f(0)).collect();
        s.push(f(u32::MAX)); // separator at index 6
        s.extend((0..6).map(|_| f(1))); // indices 7..=12
        s
    }

    // Two exact clones with a consistent small gap in both files:
    //   file 0: [0,2) gap [3,5)   file 1: [7,9) gap [11,13)
    fn groups() -> Vec<RawGroup> {
        vec![
            RawGroup {
                len: 2,
                positions: vec![0, 7],
            },
            RawGroup {
                len: 2,
                positions: vec![3, 11],
            },
        ]
    }

    #[test]
    fn stitches_consistent_gap() {
        let clusters = stitch(groups(), &spans(), 3);
        assert_eq!(clusters.len(), 1, "two adjacent clones merge into one");
        assert_eq!(clusters[0].matched, 4, "matched excludes the gap");
        assert_eq!(clusters[0].occ.len(), 2);
        // spans cover the gap: (0,5) and (7,13)
        assert!(clusters[0].occ.contains(&(0, 5)));
        assert!(clusters[0].occ.contains(&(7, 13)));
    }

    #[test]
    fn does_not_stitch_when_gap_exceeds_budget() {
        // file-1 gap is 2 (9->11); a budget of 1 leaves that occurrence
        // unfollowed, so nothing merges.
        let clusters = stitch(groups(), &spans(), 1);
        assert_eq!(clusters.len(), 2, "inconsistent/over-budget gap → no merge");
    }

    #[test]
    fn max_gap_zero_keeps_exact_clones() {
        let clusters = stitch(groups(), &spans(), 0);
        assert_eq!(clusters.len(), 2);
        assert!(clusters.iter().all(|c| c.matched == 2));
    }
}

//! Suffix-array clone extraction: build SA + LCP (Kasai), then group each
//! maximal LCP run (consecutive suffixes sharing a prefix >= threshold) into a
//! single *n-way* clone class. This replaces per-pair emission, which both
//! fragmented long clones and exploded a 3-way clone into 3 pairwise records.

use super::flatten::Master;
use bio::data_structures::suffix_array::suffix_array_int;

/// One n-way structural clone: all `positions` share a common prefix of `len`
/// tokens in the master buffer.
pub struct RawGroup {
    pub len: usize,
    pub positions: Vec<usize>,
}

/// Extract n-way clone groups of length >= `threshold`.
pub fn extract(master: &Master, threshold: usize) -> Vec<RawGroup> {
    let n = master.text.len();
    if n < 2 || threshold == 0 {
        return Vec::new();
    }
    let sa = suffix_array_int(&master.text);
    let lcp = kasai(&master.text, &sa);

    // Walk maximal runs [lo..=hi] where lcp[r] >= threshold for r in lo+1..=hi.
    // All suffixes sa[lo..=hi] then share a prefix of length min(lcp) over the
    // run — one clone class with (hi-lo+1) occurrences. Only *left-maximal*
    // runs are kept: if every occurrence is preceded by the same token, this
    // run is merely the tail of a longer clone reported one offset earlier, so
    // emitting it would fragment one logical clone into many overlapping ones.
    let mut groups = Vec::new();
    let mut r = 1;
    while r < n {
        if lcp[r] < threshold {
            r += 1;
            continue;
        }
        let lo = r - 1;
        let mut min_len = lcp[r];
        while r + 1 < n && lcp[r + 1] >= threshold {
            r += 1;
            min_len = min_len.min(lcp[r]);
        }
        let positions = &sa[lo..=r];
        if is_left_maximal(&master.text, positions) {
            groups.push(RawGroup {
                len: min_len,
                positions: positions.to_vec(),
            });
        }
        r += 1;
    }
    groups
}

/// A run is left-maximal unless all its occurrences share the same preceding
/// token (in which case it's contained in a longer clone). Position 0 has no
/// predecessor, which counts as a distinct boundary.
fn is_left_maximal(text: &[usize], positions: &[usize]) -> bool {
    let prev = |p: usize| -> i64 {
        if p == 0 {
            -1
        } else {
            text[p - 1] as i64
        }
    };
    let first = prev(positions[0]);
    positions.iter().any(|&p| prev(p) != first)
}

/// Kasai's algorithm: `lcp[r]` is the longest common prefix of `sa[r-1]`,`sa[r]`.
fn kasai(text: &[usize], sa: &[usize]) -> Vec<usize> {
    let n = text.len();
    let mut rank = vec![0usize; n];
    for (r, &p) in sa.iter().enumerate() {
        rank[p] = r;
    }
    let mut lcp = vec![0usize; n];
    let mut h = 0usize;
    for p in 0..n {
        if rank[p] == 0 {
            h = 0;
            continue;
        }
        let pred = sa[rank[p] - 1];
        while p + h < n && pred + h < n && text[p + h] == text[pred + h] {
            h += 1;
        }
        lcp[rank[p]] = h;
        h = h.saturating_sub(1);
    }
    lcp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::parser::tokens::TokenSpan;

    fn span(id: u32) -> TokenSpan {
        TokenSpan {
            file_id: id,
            start_row: 1,
            end_row: 1,
        }
    }

    #[test]
    fn groups_three_way_clone() {
        // Three identical runs [1,2,3,4] separated by unique sentinels (5,6,7)
        // + terminator 0. Alphabet is dense (0..=7), as suffix_array_int requires.
        let text = vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 6, 1, 2, 3, 4, 7, 0];
        let spans = (0..text.len()).map(|i| span(i as u32)).collect();
        let master = Master { text, spans };
        let groups = extract(&master, 4);
        assert_eq!(groups.len(), 1, "one n-way group, not pairwise records");
        assert_eq!(groups[0].len, 4);
        assert_eq!(
            groups[0].positions.len(),
            3,
            "all three occurrences grouped"
        );
    }

    #[test]
    fn nothing_below_threshold() {
        let text = vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 6, 0];
        let spans = (0..text.len()).map(|i| span(i as u32)).collect();
        let master = Master { text, spans };
        assert!(extract(&master, 5).is_empty());
    }

    /// Regression: two identical runs must collapse to ONE left-maximal group,
    /// not one group per starting offset (the fragmentation bug). Here the run
    /// [1,2,3,4] repeats; offsets 1,2,3 share the same preceding token and must
    /// be suppressed, leaving a single length-4 clone.
    #[test]
    fn left_maximal_collapses_offset_fragments() {
        let text = vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 0]; // dense alphabet 0..=5
        let spans = (0..text.len()).map(|i| span(i as u32)).collect();
        let master = Master { text, spans };
        let groups = extract(&master, 2);
        assert_eq!(
            groups.len(),
            1,
            "must not fragment into one group per offset"
        );
        assert_eq!(groups[0].len, 4);
        assert_eq!(groups[0].positions.len(), 2);
    }
}

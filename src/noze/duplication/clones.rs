//! Suffix-array clone extraction: build SA + LCP (Kasai), then group each
//! LCP interval (consecutive suffixes sharing a prefix >= threshold) into a
//! single *n-way* clone class. This replaces per-pair emission, which both
//! fragmented long clones and exploded a 3-way clone into 3 pairwise records.

use super::flatten::Master;
use bio::data_structures::suffix_array::suffix_array_int;
use libsais::SuffixArrayConstruction;

/// One n-way structural clone: all `positions` share a common prefix of `len`
/// tokens in the master buffer.
pub struct RawGroup {
    pub len: usize,
    pub positions: Vec<usize>,
}

/// Which LCP intervals [`extract`] reports.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Nesting {
    /// Only the widest interval of each run (one group per maximal run).
    Outermost,
    /// Also outermost-at-`report_len` and disjoint nested intervals (longer clones, fewer copies).
    Seeded { report_len: usize },
}

/// Extract n-way clone groups of length >= `min_len`.
pub fn extract(master: &mut Master, min_len: usize, nesting: Nesting) -> Vec<RawGroup> {
    let n = master.text.len();
    if n < 2 || min_len == 0 {
        return Vec::new();
    }
    let sa = build_suffix_array(&master.text);
    let lcp = kasai(&master.text, &sa);

    // Bottom-up LCP-interval walk: each popped `(len, left)` covers sa[left..i],
    // whose suffixes share exactly `len` tokens. Only *left-maximal* intervals
    // are kept: if every occurrence is preceded by the same token, it is the
    // tail of a longer clone reported one offset earlier.
    let mut groups = Vec::new();
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    for i in 1..=n {
        let cur = lcp.get(i).copied().unwrap_or(0);
        let mut lb = i - 1;
        while let Some(&(len, left)) = stack.last().filter(|&&(len, _)| cur < len) {
            stack.pop();
            lb = left;
            if len < min_len {
                continue;
            }
            let parent = stack.last().map_or(cur, |&(top, _)| top.max(cur));
            let positions = &sa[left..i];
            if is_reported(nesting, min_len, parent, len, positions)
                && is_left_maximal(&master.text, positions)
            {
                groups.push(RawGroup {
                    len,
                    positions: positions.to_vec(),
                });
            }
        }
        if stack.last().is_some_and(|&(len, _)| cur > len) {
            stack.push((cur, lb));
        }
    }
    groups
}

/// Whether an interval of `len` under a `parent`-long one is its own clone group.
fn is_reported(
    nesting: Nesting,
    min_len: usize,
    parent: usize,
    len: usize,
    positions: &[usize],
) -> bool {
    parent < min_len
        || match nesting {
            Nesting::Outermost => false,
            Nesting::Seeded { report_len } => {
                parent < report_len || can_be_disjoint(positions, len)
            }
        }
}

fn build_suffix_array(text: &[i32]) -> Vec<usize> {
    let mut scratch = text.to_vec();
    match SuffixArrayConstruction::for_text_mut(&mut scratch)
        .in_owned_buffer32()
        .single_threaded()
        .run()
    {
        Ok(result) => result
            .into_vec()
            .into_iter()
            .map(|position| position as usize)
            .collect(),
        Err(err) => {
            eprintln!("[sensez] suffix array: libsais failed ({err}); using portable fallback");
            // Master-buffer codes are dense and non-negative by construction
            // (see `flatten::DenseAlphabet`), so widening is lossless here.
            let fallback: Vec<usize> = text.iter().map(|&value| value as usize).collect();
            suffix_array_int(&fallback)
        }
    }
}

/// False when `len`-token occurrences at `positions` must overlap (tandem repeats).
fn can_be_disjoint(positions: &[usize], len: usize) -> bool {
    let mut sorted = positions.to_vec();
    sorted.sort_unstable();
    sorted.windows(2).all(|pair| pair[1] - pair[0] >= len)
}

/// A run is left-maximal unless all its occurrences share the same preceding
/// token (in which case it's contained in a longer clone). Position 0 has no
/// predecessor, which counts as a distinct boundary.
fn is_left_maximal(text: &[i32], positions: &[usize]) -> bool {
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
fn kasai(text: &[i32], sa: &[usize]) -> Vec<usize> {
    let n = text.len();
    let mut rank = vec![0usize; n];
    for (r, &p) in sa.iter().enumerate() {
        rank[p] = r;
    }
    let mut lcp = vec![0usize; n];
    let mut scan = PrefixScan::default();
    for p in 0..n {
        if rank[p] == 0 {
            scan.reset();
            continue;
        }
        let pred = sa[rank[p] - 1];
        lcp[rank[p]] = scan.common_len(text, p, pred);
        scan.step();
    }
    lcp
}

#[derive(Default)]
struct PrefixScan {
    len: usize,
}

impl PrefixScan {
    fn reset(&mut self) {
        self.len = 0;
    }

    fn common_len(&mut self, text: &[i32], left: usize, right: usize) -> usize {
        while left + self.len < text.len()
            && right + self.len < text.len()
            && text[left + self.len] == text[right + self.len]
        {
            self.len += 1;
        }
        self.len
    }

    fn step(&mut self) {
        self.len = self.len.saturating_sub(1);
    }
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
            start_byte: 0,
            end_byte: 0,
        }
    }

    #[test]
    fn groups_three_way_clone() {
        // Three identical runs [1,2,3,4] separated by unique sentinels (5,6,7)
        // + terminator 0. Alphabet is dense (0..=7), as suffix_array_int requires.
        let text = vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 6, 1, 2, 3, 4, 7, 0];
        let spans = (0..text.len()).map(|i| span(i as u32)).collect();
        let mut master = Master { text, spans };
        let groups = extract(&mut master, 4, Nesting::Outermost);
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
        let mut master = Master { text, spans };
        assert!(extract(&mut master, 5, Nesting::Outermost).is_empty());
    }

    /// Regression: two identical runs must collapse to ONE left-maximal group,
    /// not one group per starting offset (the fragmentation bug). Here the run
    /// [1,2,3,4] repeats; offsets 1,2,3 share the same preceding token and must
    /// be suppressed, leaving a single length-4 clone.
    #[test]
    fn left_maximal_collapses_offset_fragments() {
        let text = vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 0]; // dense alphabet 0..=5
        let spans = (0..text.len()).map(|i| span(i as u32)).collect();
        let mut master = Master { text, spans };
        let groups = extract(&mut master, 2, Nesting::Outermost);
        assert_eq!(
            groups.len(),
            1,
            "must not fragment into one group per offset"
        );
        assert_eq!(groups[0].len, 4);
        assert_eq!(groups[0].positions.len(), 2);
    }

    #[test]
    fn adjacent_overlap_is_not_disjoint_despite_a_wide_span() {
        assert!(!can_be_disjoint(&[10, 0, 1], 4));
        assert!(can_be_disjoint(&[8, 0, 4], 4));
    }

    #[test]
    fn nested_intervals_keep_longer_clones_among_fewer_occurrences() {
        // [1,2,3] occurs three times; [1,2,3,4,5] only in the last two.
        let text = vec![1, 2, 3, 9, 1, 2, 3, 4, 5, 8, 1, 2, 3, 4, 5, 7, 0];
        let spans = (0..text.len()).map(|i| span(i as u32)).collect();
        let mut master = Master { text, spans };
        let shape = |groups: Vec<RawGroup>| -> Vec<(usize, usize)> {
            let mut out: Vec<_> = groups.iter().map(|g| (g.len, g.positions.len())).collect();
            out.sort_unstable();
            out
        };
        assert_eq!(
            shape(extract(&mut master, 3, Nesting::Outermost)),
            vec![(3, 3)]
        );
        assert_eq!(
            shape(extract(&mut master, 3, Nesting::Seeded { report_len: 3 })),
            vec![(3, 3), (5, 2)]
        );
    }
}

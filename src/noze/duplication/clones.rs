use super::flatten::Master;
use bio::data_structures::suffix_array::suffix_array_int;
use libsais::SuffixArrayConstruction;

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
#[path = "clones_tests.rs"]
mod tests;

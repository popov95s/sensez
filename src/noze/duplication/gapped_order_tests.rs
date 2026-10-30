use super::*;

/// The merge pass before candidates were found in parallel: one candidate at a time, skipping consumed clusters.
fn sequential_merge_pass(clusters: &mut Vec<Cluster>, spans: &[TokenSpan], max_gap: usize) -> bool {
    let mut starts: Starts = HashMap::default();
    for (ci, c) in clusters.iter().enumerate() {
        for &(s, e) in &c.occ {
            starts.entry(s).or_default().push((ci, e));
        }
    }
    let mut consumed = vec![false; clusters.len()];
    let mut merged: Vec<Cluster> = Vec::new();
    for i in 0..clusters.len() {
        if consumed[i] {
            continue;
        }
        if let Some((j, candidate)) = stitched(clusters, i, &starts, spans, max_gap) {
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

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

fn spans(rng: &mut Rng) -> Vec<TokenSpan> {
    let span = |file_id| TokenSpan {
        file_id,
        start_row: 1,
        end_row: 1,
        start_byte: 0,
        end_byte: 0,
    };
    let mut spans = Vec::new();
    for file_id in 0..1 + rng.below(4) as u32 {
        spans.extend((0..8 + rng.below(40)).map(|_| span(file_id)));
        spans.push(span(u32::MAX));
    }
    spans
}

fn groups(rng: &mut Rng, tokens: usize) -> Vec<RawGroup> {
    (0..1 + rng.below(30))
        .map(|_| {
            let len = 1 + rng.below(4);
            let mut positions: Vec<usize> = (0..2 + rng.below(3))
                .map(|_| rng.below(tokens - len))
                .collect();
            positions.sort_unstable();
            positions.dedup();
            RawGroup { len, positions }
        })
        .collect()
}

fn shape(clusters: &[Cluster]) -> Vec<(usize, Vec<(usize, usize)>)> {
    clusters
        .iter()
        .map(|c| (c.matched, c.occ.clone()))
        .collect()
}

#[test]
fn parallel_candidates_stitch_exactly_like_the_sequential_pass() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut cases_with_merges = 0;
    for _ in 0..500 {
        let spans = spans(&mut rng);
        let groups = groups(&mut rng, spans.len());
        let max_gap = 1 + rng.below(4);
        let mut expected = stitch(clone_groups(&groups), &spans, 0);
        for _ in 0..MAX_MERGE_PASSES {
            if !sequential_merge_pass(&mut expected, &spans, max_gap) {
                break;
            }
        }

        let unstitched = groups.len();
        let actual = stitch(groups, &spans, max_gap);

        assert_eq!(shape(&actual), shape(&expected));
        cases_with_merges += usize::from(actual.len() < unstitched);
    }
    assert!(cases_with_merges >= 25, "{cases_with_merges}");
}

fn clone_groups(groups: &[RawGroup]) -> Vec<RawGroup> {
    groups
        .iter()
        .map(|g| RawGroup {
            len: g.len,
            positions: g.positions.clone(),
        })
        .collect()
}

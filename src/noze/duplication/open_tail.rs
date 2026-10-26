use super::gapped::Cluster;
use crate::spine::parser::tokens::TokenSpan;

/// The cluster without trailing tokens whose syntax node is still open where the match ends; the longest open tail trims every occurrence.
pub(super) fn close_tails(cluster: Cluster, spans: &[TokenSpan]) -> Cluster {
    let cut = cluster
        .occ
        .iter()
        .map(|&(start, end)| open_tail(spans, start, end))
        .max()
        .unwrap_or_default();
    Cluster {
        matched: cluster.matched.saturating_sub(cut),
        occ: cluster
            .occ
            .into_iter()
            .map(|(start, end)| (start, end - cut.min(end - start)))
            .collect(),
    }
}

/// Trailing tokens of `[start, end)` whose node is still open at the next token in the same file.
fn open_tail(spans: &[TokenSpan], start: usize, end: usize) -> usize {
    let limit = match spans.get(end) {
        Some(next) if next.file_id == spans[start].file_id => next.start_byte,
        _ => return 0,
    };
    (start..end)
        .rev()
        .take_while(|&token| spans[token].end_byte > limit)
        .count()
}

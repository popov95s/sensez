//! In-memory linear similarity search over the flat vector set.

use super::cache::CachedDoc;
use crate::eyez::DocKind;
use rayon::prelude::*;

/// One search result, ready to serialize for the CLI / MCP surface.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchHit {
    /// Source file the matched doc lives in.
    pub file: String,
    /// 1-indexed line of the doc element.
    pub line: usize,
    pub symbol_path: String,
    pub kind: DocKind,
    pub text: String,
    /// Cosine similarity in `[-1, 1]`; higher is closer.
    pub score: f32,
}

/// The `k` docs whose vectors are most similar to `query`.
pub fn top_k(
    vectors: &[Vec<f32>],
    docs: &[CachedDoc],
    query: &[f32],
    wanted: std::num::NonZeroUsize,
) -> Vec<SearchHit> {
    let k = wanted.get();
    let mut scored: Vec<(f32, usize)> = vectors
        .par_iter()
        .enumerate()
        .map(|(i, v)| (dot(query, v), i))
        .collect();
    if k < scored.len() {
        scored.select_nth_unstable_by(k - 1, |a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    scored.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(k)
        .map(|(score, i)| {
            let d = &docs[i];
            SearchHit {
                file: d.file.clone(),
                line: d.line,
                symbol_path: d.symbol_path.clone(),
                kind: d.kind,
                text: d.text.clone(),
                score,
            }
        })
        .collect()
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;

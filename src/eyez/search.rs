//! In-memory linear similarity search over the flat vector set.
//!
//! Vectors are L2-normalized, so cosine similarity is a plain dot product. For
//! the doc counts a codebase produces (well under 10k), a rayon sweep + partial
//! sort is sub-millisecond — no vector database needed. We rank indices and clone
//! only the `top_k` survivors (never every `symbol_path` per query).

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
pub fn top_k(vectors: &[Vec<f32>], docs: &[CachedDoc], query: &[f32], k: usize) -> Vec<SearchHit> {
    let mut scored: Vec<(f32, usize)> = vectors
        .par_iter()
        .enumerate()
        .map(|(i, v)| (dot(query, v), i))
        .collect();
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
mod tests {
    use super::*;

    fn cached(sym: &str) -> CachedDoc {
        CachedDoc {
            key: 0,
            file: "m.py".to_string(),
            line: 1,
            symbol_path: sym.to_string(),
            kind: DocKind::Docstring,
            text: sym.to_string(),
        }
    }

    /// Ranking orders by descending dot product and respects `k`.
    #[test]
    fn ranks_by_similarity_and_caps_k() {
        let docs = vec![cached("a"), cached("b"), cached("c")];
        let vectors = vec![
            vec![1.0, 0.0], // a: orthogonal to query
            vec![0.0, 1.0], // b: aligned with query
            vec![0.7, 0.7], // c: partial
        ];
        let hits = top_k(&vectors, &docs, &[0.0, 1.0], 2);
        assert_eq!(hits.len(), 2, "k caps the result count");
        assert_eq!(hits[0].symbol_path, "b", "best match first");
        assert!(hits[0].score >= hits[1].score, "scores are descending");
    }
}

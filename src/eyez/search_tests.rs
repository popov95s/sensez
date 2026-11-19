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
    let hits = top_k(
        &vectors,
        &docs,
        &[0.0, 1.0],
        std::num::NonZeroUsize::new(2).unwrap(),
    );
    assert_eq!(hits.len(), 2, "k caps the result count");
    assert_eq!(hits[0].symbol_path, "b", "best match first");
    assert!(hits[0].score >= hits[1].score, "scores are descending");
}

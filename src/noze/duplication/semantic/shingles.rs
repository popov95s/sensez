use crate::noze::duplication::near_miss::canonical_key;
use crate::spine::parser::tokens::StructuralToken;
use rustc_hash::{FxBuildHasher, FxHashSet};
use std::hash::BuildHasher;

const SHINGLE: usize = 4;

/// Hashed 4-grams, each alpha-canonicalised on its own so an edit only disturbs nearby shingles.
pub(super) fn shingles(tokens: &[(StructuralToken, u64)]) -> FxHashSet<u64> {
    tokens
        .windows(SHINGLE)
        .map(|window| FxBuildHasher.hash_one(canonical_key(window)))
        .collect()
}

/// Hashed lowercase words of a doc comment.
pub(super) fn words(text: &str) -> FxHashSet<u64> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| FxBuildHasher.hash_one(word.to_lowercase()))
        .collect()
}

pub(super) fn jaccard(left: &FxHashSet<u64>, right: &FxHashSet<u64>) -> f32 {
    let shared = left.intersection(right).count();
    let union = left.len() + right.len() - shared;
    if union == 0 {
        return 0.0;
    }
    shared as f32 / union as f32
}

#[cfg(test)]
#[path = "shingles_tests.rs"]
mod tests;

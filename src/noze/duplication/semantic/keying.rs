use std::hash::{Hash, Hasher};

/// Cache key of an embedded text, independent of where it came from.
pub(super) fn text_key(text: &str) -> u64 {
    let mut h = rustc_hash::FxHasher::default();
    "semantic-text".hash(&mut h);
    text.hash(&mut h);
    h.finish()
}

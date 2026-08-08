//! Persistent source facts cache shared by Spine's parser and pipeline.
//!
//! The cache stores only the language-neutral `Walked` projection. Graphs and
//! Noze findings remain derived values with wider invalidation scopes.

mod fingerprint;
mod store;

pub use fingerprint::SourceFingerprint;
pub use store::{CacheStats, ParseCache};

pub(crate) const PARSE_CACHE_SCHEMA: u32 = 1;

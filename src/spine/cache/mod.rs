//! Persistent source facts cache shared by Spine's parser and pipeline.
//!
//! The cache stores only the language-neutral `Walked` projection. Graphs and
//! Noze findings remain derived values with wider invalidation scopes.

mod fingerprint;
mod store;

pub use fingerprint::SourceFingerprint;
pub use store::{CacheStats, ParseCache};

// `Walked` gained/changed serialized fields while this cache was developed;
// v2 deliberately invalidates artifacts made by the earlier format. Bump this
// whenever the serialized projection or walker semantics change.
pub(crate) const PARSE_CACHE_SCHEMA: u32 = 2;

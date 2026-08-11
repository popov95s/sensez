//! Persistent source facts cache shared by Spine's parser and pipeline.
//!
//! The cache stores only the language-neutral `Walked` projection. Graphs and
//! Noze findings remain derived values with wider invalidation scopes.

mod fingerprint;
mod snapshot;

pub use fingerprint::SourceFingerprint;
pub use snapshot::{project_key, AnalysisSnapshot, SnapshotCache};

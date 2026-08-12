//! Persistent source facts cache shared by Spine's parser and pipeline.
//!
//! The cache stores only the language-neutral `Walked` projection. Graphs and
//! Noze findings remain derived values with wider invalidation scopes.

mod budget;
mod fingerprint;
mod incremental;
mod snapshot;
mod source;
mod storage;
mod writer;

pub use fingerprint::SourceFingerprint;
pub(crate) use incremental::ParseWriteInput;
pub use incremental::{ChangeStats, ParseCache, ParseCacheState};
pub use snapshot::{AnalysisSnapshot, SnapshotCache};
pub use source::{ProjectInputs, SourceFile};
pub(crate) use writer::persist;
pub use writer::{enable_background_writes, flush_background_writes, shutdown_background_writes};

pub fn load_project(
    files: &[std::path::PathBuf],
    config_signature: u64,
) -> anyhow::Result<ProjectInputs> {
    source::load(files, config_signature, snapshot::revision())
}

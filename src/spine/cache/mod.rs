//! Persistent source facts cache shared by Spine's parser and pipeline.
//!
//! The cache stores only the language-neutral `Walked` projection. Graphs and
//! Noze findings remain derived values with wider invalidation scopes.

mod fingerprint;
mod incremental;
mod source;

pub use fingerprint::SourceFingerprint;
pub use incremental::{ChangeStats, ParseCacheState};
pub use source::{ProjectInputs, SourceFile};

pub fn load_project(files: &[std::path::PathBuf]) -> anyhow::Result<ProjectInputs> {
    source::load(files)
}

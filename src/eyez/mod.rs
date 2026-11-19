//! Indexes a project's docstrings and comments and answers intent queries by embedding similarity.

mod cache;
pub(crate) mod capture;
mod docs;
mod embed;
pub(crate) mod extract;
mod index;
mod model;
mod reindex;
mod search;
pub(crate) mod semantic_cache;
mod top_k;
mod vectors;

pub use docs::{DocKind, RawDoc};
pub use index::Index;
pub use top_k::DEFAULT_TOP_K;

pub(crate) use reindex::{reindex, Rebuild, Warmup};
pub(crate) use vectors::{embed_texts, semantic_vectors};

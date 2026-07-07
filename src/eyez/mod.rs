//! Indexes a project's docstrings + comments (captured during the language walk
//! into [`Walked::docs`](crate::spine::parser::Walked)) and answers intent queries via
//! local CPU embeddings + an in-memory linear similarity sweep. State persists in
//! a single bincode cache under `.sensez/`; only new/changed docs are re-embedded.

mod cache;
pub(crate) mod capture;
mod docs;
mod embed;
pub(crate) mod extract;
mod search;
pub(crate) mod semantic_cache;

pub use docs::{DocKind, RawDoc};
pub use search::SearchHit;

use anyhow::{Context, Result};
use std::path::Path;

/// An in-memory eyez index over one project's documentation.
pub struct Index {
    embedder: embed::Embedder,
    cache: cache::SystemCache,
}

pub(crate) struct ReindexReport {
    pub docs: usize,
    pub semantic_warmed: bool,
}

pub(crate) fn embed_texts(texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
    Ok(embed::Embedder::load()?.embed(texts))
}

pub(crate) fn semantic_vectors(
    root: &Path,
    inputs: &[semantic_cache::BundleInput],
) -> anyhow::Result<Vec<Vec<f32>>> {
    semantic_cache::vectors(root, inputs)
}

pub(crate) fn reindex(root: &Path, force: bool, semantic: bool) -> Result<ReindexReport> {
    if force {
        cache::clear(root)?;
        semantic_cache::clear(root)?;
    }
    let index = Index::open(root)?;
    if semantic {
        let _ = crate::analyze_path(root, None, None)?;
    }
    Ok(ReindexReport {
        docs: index.len(),
        semantic_warmed: semantic,
    })
}

impl Index {
    /// Open (or build) the index for `root`: load the embedding model, hydrate the
    /// on-disk cache, diff current docs against it by content key, embed only the
    /// new/changed ones, and persist the merged cache.
    pub fn open(root: &Path) -> Result<Self> {
        let embedder = embed::Embedder::load().context("loading embedding model")?;
        let mut cache = cache::load(root);
        let docs = extract::collect(root).context("collecting project docs")?;
        cache.refresh(&docs, &embedder)?;
        cache.persist(root)?;
        Ok(Index { embedder, cache })
    }

    /// The `top_k` indexed docs most semantically similar to `query`.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<SearchHit> {
        let q = self.embedder.embed_one(query);
        search::top_k(&self.cache.vectors, &self.cache.docs, &q, top_k)
    }

    /// Number of indexed documents.
    pub fn len(&self) -> usize {
        self.cache.docs.len()
    }
}

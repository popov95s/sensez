use super::search::{self, SearchHit};
use super::{cache, embed, extract};
use anyhow::{Context, Result};
use std::path::Path;

/// An in-memory eyez index over one project's documentation.
pub struct Index {
    embedder: std::sync::Arc<embed::Embedder>,
    cache: cache::SystemCache,
}

impl Index {
    /// Open (or build) the index for `root`.
    pub fn open(root: &Path) -> Result<Self> {
        let embedder = embed::Embedder::load().context("loading embedding model")?;
        let mut cache = cache::load(root);
        let docs = extract::collect(root).context("collecting project docs")?;
        cache.refresh(&docs, root, &embedder)?;
        cache.persist(root)?;
        Ok(Index { embedder, cache })
    }

    /// The `top_k` indexed docs most semantically similar to `query`.
    pub fn search(&self, query: &str, top_k: std::num::NonZeroUsize) -> Vec<SearchHit> {
        let q = self.embedder.embed_one(query);
        search::top_k(&self.cache.vectors, &self.cache.docs, &q, top_k)
    }

    /// Number of indexed documents.
    pub fn len(&self) -> usize {
        self.cache.docs.len()
    }
}

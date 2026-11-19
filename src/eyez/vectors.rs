use super::{embed, semantic_cache};
use anyhow::Result;
use std::path::Path;

pub(crate) fn embed_texts(texts: &[String]) -> Result<Vec<Vec<f32>>> {
    Ok(embed::Embedder::load()?.embed(texts))
}

pub(crate) fn semantic_vectors(
    root: &Path,
    inputs: &[semantic_cache::BundleInput<'_>],
) -> Result<Vec<Vec<f32>>> {
    semantic_cache::vectors(root, inputs)
}

//! Persistent vector cache for semantic-duplication comment bundles.
//!
//! This cache is separate from raw doc search because semantic duplication
//! indexes per-symbol bundles, not individual comments/docstrings.

use super::cache;
use super::embed::{Embedder, MODEL_ID};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

const CACHE_REL: &str = ".sensez/semantic-dup-cache.bin";
const SCHEMA_VERSION: u32 = 1;

#[derive(Clone)]
pub(crate) struct BundleInput {
    pub key: u64,
    pub text: String,
}

#[derive(Default, Serialize, Deserialize)]
struct SemanticCache {
    schema_version: u32,
    model_id: String,
    vectors: Vec<CachedVector>,
}

#[derive(Serialize, Deserialize)]
struct CachedVector {
    key: u64,
    vector: Vec<f32>,
}

pub(crate) fn vectors(root: &Path, inputs: &[BundleInput]) -> Result<Vec<Vec<f32>>> {
    let mut cache = load(root);
    cache.reset_if_stale();

    let mut existing: HashMap<u64, Vec<f32>> = cache
        .vectors
        .into_iter()
        .map(|cached| (cached.key, cached.vector))
        .collect();

    let mut seen = HashSet::new();
    let mut wanted = Vec::new();
    let mut missing = Vec::new();
    for input in inputs {
        if seen.insert(input.key) {
            if !existing.contains_key(&input.key) {
                missing.push(input);
            }
            wanted.push(input);
        }
    }

    if !missing.is_empty() {
        let embedder = Embedder::load().context("loading semantic-duplication embedder")?;
        let mut keys = Vec::with_capacity(missing.len());
        let mut texts = Vec::with_capacity(missing.len());
        for input in missing {
            keys.push(input.key);
            texts.push(input.text.clone());
        }
        let embedded = embedder.embed(&texts);
        if embedded.len() != keys.len() {
            return Err(anyhow::anyhow!(
                "embedder returned {}/{} vectors",
                embedded.len(),
                keys.len()
            ));
        }
        for (key, vector) in keys.into_iter().zip(embedded) {
            existing.insert(key, vector);
        }
    }

    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        let Some(vector) = existing.get(&input.key) else {
            anyhow::bail!(
                "no vector resolved for bundle key {:#x}; cache state is inconsistent",
                input.key
            );
        };
        out.push(vector.clone());
    }
    let mut next_cache = SemanticCache {
        schema_version: SCHEMA_VERSION,
        model_id: MODEL_ID.to_string(),
        vectors: Vec::with_capacity(wanted.len()),
    };
    for input in wanted {
        if let Some(vector) = existing.remove(&input.key) {
            next_cache.vectors.push(CachedVector {
                key: input.key,
                vector,
            });
        }
    }
    persist(root, &next_cache)?;
    Ok(out)
}

pub(crate) fn clear(root: &Path) -> Result<()> {
    cache::remove_file(root, CACHE_REL)
}

fn load(root: &Path) -> SemanticCache {
    std::fs::read(root.join(CACHE_REL))
        .ok()
        .and_then(|bytes| postcard::from_bytes(&bytes).ok())
        .unwrap_or_default()
}

fn persist(root: &Path, cache: &SemanticCache) -> Result<()> {
    use super::cache::write_atomic;
    crate::dotdir::ensure(root, None)?;
    let path = root.join(CACHE_REL);
    let bytes = postcard::to_allocvec(cache).context("serializing semantic duplication cache")?;
    write_atomic(&path, &bytes)
}

impl SemanticCache {
    fn reset_if_stale(&mut self) {
        if self.schema_version != SCHEMA_VERSION || self.model_id != MODEL_ID {
            self.schema_version = SCHEMA_VERSION;
            self.model_id = MODEL_ID.to_string();
            self.vectors.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_model_or_schema_drops_vectors() {
        let mut cache = SemanticCache {
            schema_version: 0,
            model_id: "old-model".to_string(),
            vectors: vec![CachedVector {
                key: 1,
                vector: vec![1.0],
            }],
        };

        cache.reset_if_stale();

        assert_eq!(cache.schema_version, SCHEMA_VERSION);
        assert_eq!(cache.model_id, MODEL_ID);
        assert!(cache.vectors.is_empty());
    }
}

//! Embedding backend. Default: Model2Vec `potion-retrieval-32M` — a pure-Rust
//! static embedding model (token lookup + pool, no ONNX runtime), downloaded
//! from the HuggingFace Hub on first use and cached by the `hf-hub` layer.
//!
//! Embeddings are L2-normalized (`normalize = Some(true)`), so a dot product over
//! two vectors equals their cosine similarity — what the linear search relies on.

use anyhow::{anyhow, Result};
use model2vec_rs::model::StaticModel;
use std::sync::{Arc, OnceLock};

/// HuggingFace repo id for the default static retrieval model.
pub(crate) const MODEL_ID: &str = "minishlab/potion-retrieval-32M";

/// Loaded embedding model. Cheap to call; load once and reuse.
pub struct Embedder {
    model: StaticModel,
}

static CACHED: OnceLock<Arc<Embedder>> = OnceLock::new();

impl Embedder {
    /// The shared process-wide embedder (downloading on first use).
    pub fn load() -> Result<Arc<Self>> {
        if let Some(cached) = CACHED.get() {
            return Ok(Arc::clone(cached));
        }
        let built = Arc::new(Embedder { model: build()? });
        let _ = CACHED.set(Arc::clone(&built));
        Ok(match CACHED.get() {
            Some(cached) => Arc::clone(cached),
            None => built,
        })
    }

    pub fn model_id(&self) -> &'static str {
        MODEL_ID
    }

    /// Embed many texts. Empty input yields an empty result (no model call).
    pub fn embed(&self, texts: &[String]) -> Vec<Vec<f32>> {
        if texts.is_empty() {
            return Vec::new();
        }
        self.model.encode(texts)
    }

    /// Embed a single query string.
    pub fn embed_one(&self, text: &str) -> Vec<f32> {
        self.model.encode_single(text)
    }
}

fn build() -> Result<StaticModel> {
    StaticModel::from_pretrained(MODEL_ID, None, Some(true), None)
        .map_err(|e| anyhow!("loading Model2Vec model {MODEL_ID}: {e}"))
}

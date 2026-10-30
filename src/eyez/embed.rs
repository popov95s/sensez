//! Embedding backend. Default: Model2Vec `potion-code-16M-v2` — a pure-Rust
//! static embedding model (token lookup + pool, no ONNX runtime), located or
//! downloaded by [`super::model`].
//!
//! Embeddings are L2-normalized (`normalize = Some(true)`), so a dot product over
//! two vectors equals their cosine similarity — what the linear search relies on.

use anyhow::{anyhow, Result};
use model2vec_rs::model::StaticModel;
use std::sync::Arc;

/// HuggingFace repo id for the default static code embedding model.
pub(crate) const MODEL_ID: &str = "minishlab/potion-code-16M-v2";

/// Loaded embedding model, dropped with its last handle.
pub struct Embedder {
    model: StaticModel,
}

impl Embedder {
    /// Load the model from disk (downloading it on first use).
    pub fn load() -> Result<Arc<Self>> {
        Ok(Arc::new(Embedder { model: build()? }))
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
    StaticModel::from_pretrained(super::model::model_dir()?, None, Some(true), None)
        .map_err(|e| anyhow!("loading Model2Vec model {MODEL_ID}: {e}"))
}

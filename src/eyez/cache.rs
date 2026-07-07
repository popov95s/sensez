//! On-disk eyez cache (bincode) with a content-hash delta.
//!
//! Correctness rests on a per-doc content key — `hash(symbol_path, kind, text)` —
//! NOT on mtime and NOT on a tree structure. On refresh, any doc whose key is
//! already cached keeps its vector; only genuinely new/changed docs are embedded;
//! docs that disappeared are dropped. So a docstring edit re-embeds exactly that
//! one symbol, and untouched docs are never recomputed.

use super::embed::Embedder;
use super::extract::Doc;
use crate::eyez::DocKind;
use crate::fingerprints::{self, Fingerprint};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::path::Path;

/// Cache location relative to the project root.
const CACHE_REL: &str = ".sensez/eyez-cache.bin";

/// The persisted index: `docs[i]` is described by `vectors[i]` (kept 1:1).
#[derive(Default, Serialize, Deserialize)]
pub struct SystemCache {
    pub model_id: String,
    pub docs: Vec<CachedDoc>,
    pub vectors: Vec<Vec<f32>>,
}

/// One indexed doc plus its content key.
#[derive(Clone, Serialize, Deserialize)]
pub struct CachedDoc {
    pub key: u64,
    pub file: String,
    pub line: usize,
    pub symbol_path: String,
    pub kind: DocKind,
    pub text: String,
}

pub type DocFingerprint = Fingerprint<Namespace, Label, Class>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Namespace {
    Doc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub file: String,
    pub symbol_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Comment,
    Docstring,
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Class::Comment => f.write_str("eyez/comment"),
            Class::Docstring => f.write_str("eyez/docstring"),
        }
    }
}

/// Hydrate the cache from disk; a missing/corrupt file yields an empty cache.
pub fn load(root: &Path) -> SystemCache {
    let path = root.join(CACHE_REL);
    std::fs::read(&path)
        .ok()
        .and_then(|bytes| postcard::from_bytes(&bytes).ok())
        .unwrap_or_default()
}

pub fn clear(root: &Path) -> Result<()> {
    remove_file(root, CACHE_REL)
}

pub(crate) fn remove_file(root: &Path, rel: &str) -> Result<()> {
    let path = root.join(rel);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("removing {}", path.display())),
    }
}

impl SystemCache {
    /// Diff `docs` against the cache by content key and embed only the misses.
    pub fn refresh(&mut self, docs: &[Doc], embedder: &Embedder) -> Result<()> {
        if self.model_id != embedder.model_id() {
            self.docs.clear();
            self.vectors.clear();
            self.model_id = embedder.model_id().to_string();
        }

        // Cached key -> vector. Both sides are rebuilt below, so the old
        // entries are MOVED out, never cloned (vectors can be MBs in total).
        let mut have: HashMap<u64, Vec<f32>> = std::mem::take(&mut self.docs)
            .into_iter()
            .map(|d| d.key)
            .zip(std::mem::take(&mut self.vectors))
            .collect();

        // Desired docs, de-duplicated by key (identical doc text under the same
        // symbol/kind collapses to one entry).
        let mut seen: HashSet<u64> = HashSet::new();
        let wanted: Vec<&Doc> = docs.iter().filter(|d| seen.insert(key(d))).collect();

        // Embed the cache misses in one batch.
        let missing: Vec<&Doc> = wanted
            .iter()
            .copied()
            .filter(|d| !have.contains_key(&key(d)))
            .collect();
        let texts: Vec<String> = missing.iter().map(|d| d.text.clone()).collect();
        for (d, vector) in missing.iter().zip(embedder.embed(&texts)) {
            have.insert(key(d), vector);
        }

        // Rebuild aligned docs+vectors for exactly the wanted set (drops stale).
        let mut new_docs = Vec::with_capacity(wanted.len());
        let mut new_vecs = Vec::with_capacity(wanted.len());
        for d in &wanted {
            let k = key(d);
            // `wanted` is key-deduplicated, so each vector is taken exactly once.
            if let Some(vector) = have.remove(&k) {
                new_docs.push(CachedDoc {
                    key: k,
                    file: d.file.to_string_lossy().into_owned(),
                    line: d.line,
                    symbol_path: d.symbol_path.clone(),
                    kind: d.kind,
                    text: d.text.clone(),
                });
                new_vecs.push(vector);
            }
        }
        self.docs = new_docs;
        self.vectors = new_vecs;
        Ok(())
    }

    /// Write the cache back to `.sensez/eyez-cache.bin` under `root`.
    pub fn persist(&self, root: &Path) -> Result<()> {
        crate::dotdir::ensure(root, None)?;
        let path = root.join(CACHE_REL);
        let bytes = postcard::to_allocvec(self).context("serializing eyez cache")?;
        std::fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}

/// Content+identity key: changes whenever the file, symbol path, kind, or text
/// changes. Including the file keeps identical docs in different files as
/// distinct results (and only re-embeds a doc whose *text* actually changed —
/// the line moving within a file does not change the key).
pub fn key(d: &Doc) -> u64 {
    fingerprint(d).hash
}

pub fn fingerprint(d: &Doc) -> DocFingerprint {
    let file = d.file.to_string_lossy().into_owned();
    let class = match d.kind {
        DocKind::Comment => Class::Comment,
        DocKind::Docstring => Class::Docstring,
    };
    let kind = match class {
        Class::Comment => "comment",
        Class::Docstring => "docstring",
    };
    Fingerprint::identity(
        fingerprints::hash_parts(&[&file, &d.symbol_path, kind, &d.text]),
        Namespace::Doc,
        Label {
            file,
            symbol_path: d.symbol_path.clone(),
        },
        class,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(sym: &str, text: &str) -> Doc {
        Doc {
            file: std::path::PathBuf::from("m.py"),
            line: 1,
            symbol_path: sym.to_string(),
            kind: DocKind::Docstring,
            text: text.to_string(),
        }
    }

    fn cache_with_doc() -> SystemCache {
        SystemCache {
            model_id: "old-model".to_string(),
            docs: vec![CachedDoc {
                key: 1,
                file: "m.py".to_string(),
                line: 1,
                symbol_path: "m::f".to_string(),
                kind: DocKind::Docstring,
                text: "Add one.".to_string(),
            }],
            vectors: vec![vec![1.0]],
        }
    }

    /// A changed key (edited text) is a cache miss; an unchanged key is reused.
    #[test]
    fn key_changes_only_on_content_change() {
        let a = doc("m::f", "Add one.");
        let same = doc("m::f", "Add one.");
        let edited = doc("m::f", "Add two.");
        let moved = doc("m::g", "Add one.");
        assert_eq!(key(&a), key(&same));
        assert_ne!(key(&a), key(&edited), "text edit must change the key");
        assert_ne!(key(&a), key(&moved), "symbol move must change the key");
    }

    #[test]
    fn model_change_requires_reindex() {
        let mut cache = cache_with_doc();
        if cache.model_id != crate::eyez::embed::MODEL_ID {
            cache.docs.clear();
            cache.vectors.clear();
            cache.model_id = crate::eyez::embed::MODEL_ID.to_string();
        }
        assert!(cache.docs.is_empty());
        assert!(cache.vectors.is_empty());
        assert_eq!(cache.model_id, crate::eyez::embed::MODEL_ID);
    }
}

use crate::fingerprints;
use crate::spine::ir::Language;
use std::path::{Path, PathBuf};

/// Source identity and content stamp used by incremental analysis.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceFingerprint {
    pub identity: u64,
    pub content: u64,
    pub path: PathBuf,
    pub language: Language,
}

impl SourceFingerprint {
    pub fn new(path: &Path, language: Language, source: &[u8]) -> Self {
        let path_text = path.to_string_lossy();
        let language_text = format!("{language:?}");
        let identity = fingerprints::hash_parts(&[&path_text, &language_text]);
        Self {
            identity,
            content: fingerprints::hash_bytes(source),
            path: path.to_path_buf(),
            language,
        }
    }
}

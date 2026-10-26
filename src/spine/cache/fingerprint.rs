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
    #[cfg(test)]
    pub fn new(path: &Path, language: Language, source: &[u8]) -> Self {
        Self::with_content_hash(path, language, fingerprints::hash_bytes(source))
    }

    pub fn with_content_hash(path: &Path, language: Language, content: u64) -> Self {
        let path_text = path.to_string_lossy();
        let identity = fingerprints::hash_parts(&[&path_text, &format!("{language:?}")]);
        Self {
            identity,
            content,
            path: path.to_path_buf(),
            language,
        }
    }
}

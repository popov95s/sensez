use crate::fingerprints;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub content_hash: u64,
}

pub struct ProjectInputs {
    pub key: u64,
    pub sources: Vec<SourceFile>,
}

pub fn load(files: &[PathBuf], config_signature: u64, revision: &str) -> Result<ProjectInputs> {
    let sources: Result<Vec<_>> = files
        .par_iter()
        .map(|path| {
            let bytes =
                std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            let content_hash = fingerprints::hash_bytes(&bytes);
            Ok(SourceFile {
                path: path.clone(),
                bytes,
                content_hash,
            })
        })
        .collect();
    let sources = sources?;
    let mut hasher = rustc_hash::FxHasher::default();
    config_signature.hash(&mut hasher);
    revision.hash(&mut hasher);
    for source in &sources {
        source.path.hash(&mut hasher);
        source.content_hash.hash(&mut hasher);
    }
    Ok(ProjectInputs {
        key: hasher.finish(),
        sources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_changes_with_source_config_or_revision() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("a.py");
        std::fs::write(&file, "x = 1\n").unwrap();
        let first = load(std::slice::from_ref(&file), 1, "a").unwrap().key;
        std::fs::write(&file, "x = 2\n").unwrap();
        assert_ne!(
            first,
            load(std::slice::from_ref(&file), 1, "a").unwrap().key
        );
        assert_ne!(
            first,
            load(std::slice::from_ref(&file), 2, "a").unwrap().key
        );
        assert_ne!(first, load(&[file], 1, "b").unwrap().key);
    }
}

use crate::fingerprints;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub content_hash: u64,
}

pub struct ProjectInputs {
    pub sources: Vec<SourceFile>,
}

pub fn load(files: &[PathBuf]) -> Result<ProjectInputs> {
    let sources: Result<Vec<_>> = files
        .par_iter()
        .map(|path| {
            let read_started =
                crate::spine::parser::timing::enabled().then(std::time::Instant::now);
            let bytes =
                std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            if let Some(started) = read_started {
                crate::spine::parser::timing::record_read(started.elapsed());
            }
            let hash_started =
                crate::spine::parser::timing::enabled().then(std::time::Instant::now);
            let content_hash = fingerprints::hash_bytes(&bytes);
            if let Some(started) = hash_started {
                crate::spine::parser::timing::record_hash(started.elapsed());
            }
            Ok(SourceFile {
                path: path.clone(),
                bytes,
                content_hash,
            })
        })
        .collect();
    let sources = sources?;
    Ok(ProjectInputs { sources })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_source_content_hashes() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("a.py");
        std::fs::write(&file, "x = 1\n").unwrap();
        let first = load(std::slice::from_ref(&file)).unwrap();
        assert_eq!(
            first.sources[0].content_hash,
            fingerprints::hash_bytes(b"x = 1\n")
        );
        std::fs::write(&file, "x = 2\n").unwrap();
        let second = load(&[file]).unwrap();
        assert_ne!(
            first.sources[0].content_hash,
            second.sources[0].content_hash
        );
    }
}

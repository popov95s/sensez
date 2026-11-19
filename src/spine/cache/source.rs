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
#[path = "source_tests.rs"]
mod tests;

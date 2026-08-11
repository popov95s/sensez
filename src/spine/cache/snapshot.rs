use crate::fingerprints;
use crate::report::AnalysisReport;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const CACHE_REL: &str = ".sensez/analysis-v1.bin";
const LEGACY_PARSE_REL: &str = ".sensez/parse-v1";
const SCHEMA: u32 = 1;
const MAX_BYTES: usize = 1_000_000;
const MAX_DECOMPRESSED_BYTES: u64 = 32_000_000;
const REVISION: &str = concat!(env!("CARGO_PKG_VERSION"), "-analysis-v1");

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisSnapshot {
    pub report: AnalysisReport,
    pub module_files: HashMap<String, PathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Artifact {
    schema: u32,
    revision: String,
    key: u64,
    snapshot: AnalysisSnapshot,
}

pub struct SnapshotCache {
    path: PathBuf,
}

impl SnapshotCache {
    pub fn new(root: &Path) -> Self {
        let _ = fs::remove_dir_all(root.join(LEGACY_PARSE_REL));
        Self {
            path: root.join(CACHE_REL),
        }
    }

    pub fn load(&self, key: u64) -> Option<AnalysisSnapshot> {
        let bytes = fs::read(&self.path).ok()?;
        if bytes.len() > MAX_BYTES {
            return None;
        }
        let mut decoded = Vec::new();
        GzDecoder::new(bytes.as_slice())
            .take(MAX_DECOMPRESSED_BYTES)
            .read_to_end(&mut decoded)
            .ok()?;
        let artifact: Artifact = serde_json::from_slice(&decoded).ok()?;
        (artifact.schema == SCHEMA && artifact.revision == REVISION && artifact.key == key)
            .then_some(artifact.snapshot)
    }

    pub fn persist(&self, key: u64, snapshot: &AnalysisSnapshot) -> Result<bool> {
        let artifact = Artifact {
            schema: SCHEMA,
            revision: REVISION.to_string(),
            key,
            snapshot: snapshot.clone(),
        };
        let serialized = serde_json::to_vec(&artifact).context("serializing analysis cache")?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&serialized)?;
        let bytes = encoder.finish()?;
        if bytes.len() > MAX_BYTES {
            return Ok(false);
        }
        let Some(directory) = self.path.parent() else {
            return Ok(false);
        };
        fs::create_dir_all(directory)?;
        let temp = self
            .path
            .with_extension(format!("tmp-{}", std::process::id()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&temp)?;
            file.write_all(&bytes)?;
            fs::rename(&temp, &self.path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result.context("persisting analysis cache")?;
        Ok(true)
    }
}

pub fn project_key(files: &[PathBuf], config_signature: u64) -> Result<u64> {
    let parts: Result<Vec<_>> = files
        .par_iter()
        .map(|path| {
            let source = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            Ok((path.clone(), fingerprints::hash_bytes(&source)))
        })
        .collect();
    let mut hasher = rustc_hash::FxHasher::default();
    config_signature.hash(&mut hasher);
    REVISION.hash(&mut hasher);
    for (path, content) in parts? {
        path.hash(&mut hasher);
        content.hash(&mut hasher);
    }
    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_under_the_hard_cap() {
        let root = tempfile::tempdir().unwrap();
        let cache = SnapshotCache::new(root.path());
        let snapshot = AnalysisSnapshot {
            report: AnalysisReport::default(),
            module_files: HashMap::new(),
        };
        assert!(cache.persist(7, &snapshot).unwrap());
        assert!(cache.load(7).is_some());
        assert!(cache.load(8).is_none());
        assert!(fs::metadata(root.path().join(CACHE_REL)).unwrap().len() < MAX_BYTES as u64);
    }

    #[test]
    fn opening_snapshot_cache_removes_legacy_parse_artifacts() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join(LEGACY_PARSE_REL);
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("artifact.bin"), b"obsolete").unwrap();

        let _cache = SnapshotCache::new(root.path());

        assert!(!legacy.exists());
    }

    #[test]
    fn project_key_changes_with_source_or_config() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("a.py");
        fs::write(&file, "x = 1\n").unwrap();
        let first = project_key(std::slice::from_ref(&file), 1).unwrap();
        fs::write(&file, "x = 2\n").unwrap();
        assert_ne!(first, project_key(std::slice::from_ref(&file), 1).unwrap());
        assert_ne!(first, project_key(&[file], 2).unwrap());
    }
}

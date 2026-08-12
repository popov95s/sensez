use crate::report::{AnalysisReport, Severity};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

const CACHE_REL: &str = ".sensez/analysis-v1.bin";
const LEGACY_PARSE_REL: &str = ".sensez/parse-v1";
const SCHEMA: u32 = 2;
const MAX_DECOMPRESSED_BYTES: u64 = 32_000_000;
const REVISION: &str = concat!(env!("CARGO_PKG_VERSION"), "-analysis-v2");
// Keep JSON inside gzip: representative schema benchmarking found compressed
// MessagePack 13% larger, while JSON preserves the public report's conditional
// Serde fields without a second cache-only model.

pub(super) fn revision() -> &'static str {
    REVISION
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisSnapshot {
    pub report: AnalysisReport,
    pub module_files: HashMap<String, PathBuf>,
    smell_end_lines: Vec<usize>,
    smell_severities: Vec<Severity>,
}

impl AnalysisSnapshot {
    pub fn new(report: AnalysisReport, module_files: HashMap<String, PathBuf>) -> Self {
        let smell_end_lines = report
            .smells
            .iter()
            .map(|finding| finding.end_line)
            .collect();
        let smell_severities = report
            .smells
            .iter()
            .map(|finding| finding.severity)
            .collect();
        Self {
            report,
            module_files,
            smell_end_lines,
            smell_severities,
        }
    }

    fn restore_internal_fields(&mut self) {
        for ((finding, end_line), severity) in self
            .report
            .smells
            .iter_mut()
            .zip(&self.smell_end_lines)
            .zip(&self.smell_severities)
        {
            finding.end_line = *end_line;
            finding.severity = *severity;
        }
        self.report.meta.glossary = crate::noze::glossary::for_report(&self.report);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Artifact {
    schema: u32,
    revision: String,
    key: u64,
    snapshot: AnalysisSnapshot,
}

#[derive(Clone)]
pub struct SnapshotCache {
    path: PathBuf,
}

pub(super) struct PreparedSnapshot {
    bytes: Vec<u8>,
}

impl SnapshotCache {
    pub fn new(root: &Path) -> Self {
        let _ = fs::remove_dir_all(root.join(LEGACY_PARSE_REL));
        super::budget::enforce_total(root);
        let path = root.join(CACHE_REL);
        Self { path }
    }

    pub fn load(&self, key: u64) -> Option<AnalysisSnapshot> {
        let bytes = timed("cache-read", || fs::read(&self.path)).ok()?;
        if bytes.len() > super::budget::TOTAL_BYTES {
            return None;
        }
        let decoded = timed("cache-decompress", || {
            let mut decoded = Vec::new();
            GzDecoder::new(bytes.as_slice())
                .take(MAX_DECOMPRESSED_BYTES)
                .read_to_end(&mut decoded)
                .map(|_| decoded)
        })
        .ok()?;
        let artifact: Artifact = timed("cache-decode", || serde_json::from_slice(&decoded)).ok()?;
        if artifact.schema != SCHEMA || artifact.revision != REVISION || artifact.key != key {
            return None;
        }
        let mut snapshot = artifact.snapshot;
        snapshot.restore_internal_fields();
        Some(snapshot)
    }

    pub(super) fn path_key(&self) -> PathBuf {
        self.path.clone()
    }

    pub(super) fn prepare(
        key: u64,
        snapshot: &AnalysisSnapshot,
    ) -> Result<Option<PreparedSnapshot>> {
        let artifact = Artifact {
            schema: SCHEMA,
            revision: REVISION.to_string(),
            key,
            snapshot: snapshot.clone(),
        };
        let serialized = timed("cache-encode", || serde_json::to_vec(&artifact))
            .context("serializing analysis cache")?;
        let bytes = timed("cache-compress", || {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(&serialized)?;
            encoder.finish()
        })?;
        if bytes.len() > super::budget::TOTAL_BYTES {
            return Ok(None);
        }
        Ok(Some(PreparedSnapshot { bytes }))
    }

    pub(super) fn write(&self, prepared: PreparedSnapshot) -> Result<()> {
        timed("cache-write", || {
            super::storage::atomic_write(&self.path, &prepared.bytes, "persisting analysis cache")
        })
    }

    pub(super) fn remove(&self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl PreparedSnapshot {
    pub(super) fn len(&self) -> usize {
        self.bytes.len()
    }
}

fn timed<T>(label: &str, operation: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let result = operation();
    if std::env::var_os("SENSEZ_TIMING").is_some() {
        eprintln!(
            "[timing] {label:<16} {:>7.1}ms",
            started.elapsed().as_secs_f64() * 1e3
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprints;

    #[test]
    fn snapshot_round_trips_under_the_hard_cap() {
        let root = tempfile::tempdir().unwrap();
        let cache = SnapshotCache::new(root.path());
        let snapshot = AnalysisSnapshot::new(AnalysisReport::default(), HashMap::new());
        cache
            .write(SnapshotCache::prepare(7, &snapshot).unwrap().unwrap())
            .unwrap();
        assert!(cache.load(7).is_some());
        assert!(cache.load(8).is_none());
        assert!(
            fs::metadata(root.path().join(CACHE_REL)).unwrap().len()
                < super::super::budget::TOTAL_BYTES as u64
        );
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
    fn oversized_snapshot_is_not_persisted() {
        let root = tempfile::tempdir().unwrap();
        let _cache = SnapshotCache::new(root.path());
        let module_files = (0_u64..120_000)
            .map(|value| {
                let digest = fingerprints::hash_bytes(&value.to_le_bytes());
                (
                    format!("module-{digest:016x}"),
                    PathBuf::from(format!("/{digest:016x}.ts")),
                )
            })
            .collect();
        let snapshot = AnalysisSnapshot::new(AnalysisReport::default(), module_files);

        assert!(SnapshotCache::prepare(7, &snapshot).unwrap().is_none());
        assert!(!root.path().join(CACHE_REL).exists());
    }
}

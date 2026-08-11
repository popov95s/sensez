use crate::spine::ir::{Language, Walked};
use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const CACHE_DIR: &str = ".sensez/parse-v1";
const SCHEMA_VERSION: u32 = crate::spine::cache::PARSE_CACHE_SCHEMA;
// Bump the suffix when parser/walker semantics change without an IR layout
// change. The package version covers released grammar revisions.
const PARSER_REVISION: &str = concat!(env!("CARGO_PKG_VERSION"), "-walk-v1");

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Artifact {
    schema_version: u32,
    parser_revision: String,
    identity: u64,
    content: u64,
    language: Language,
    walked: Walked,
}

/// On-disk cache for immutable per-file parser output.
#[derive(Debug, Clone)]
pub struct ParseCache {
    root: PathBuf,
    stats: Arc<CacheStats>,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl CacheStats {
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }
    pub fn misses(&self) -> usize {
        self.misses.load(Ordering::Relaxed)
    }
}

impl ParseCache {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            stats: Arc::new(CacheStats::default()),
        }
    }

    pub fn stats(&self) -> Arc<CacheStats> {
        Arc::clone(&self.stats)
    }

    pub(crate) fn load(
        &self,
        identity: u64,
        content: u64,
        language: Language,
        key: u64,
    ) -> Option<Walked> {
        let bytes = match fs::read(self.path_for(key)) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };
        let artifact: Artifact = match postcard::from_bytes(&bytes) {
            Ok(artifact) => artifact,
            Err(_) => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };
        if artifact.schema_version != SCHEMA_VERSION
            || artifact.parser_revision != PARSER_REVISION
            || artifact.identity != identity
            || artifact.content != content
            || artifact.language != language
        {
            self.stats.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        self.stats.hits.fetch_add(1, Ordering::Relaxed);
        Some(artifact.walked)
    }

    pub(crate) fn persist(
        &self,
        identity: u64,
        content: u64,
        language: Language,
        key: u64,
        walked: &Walked,
    ) -> Result<()> {
        let directory = self.root.join(CACHE_DIR);
        fs::create_dir_all(&directory)
            .with_context(|| format!("creating {}", directory.display()))?;
        let path = self.path_for(key);
        let artifact = Artifact {
            schema_version: SCHEMA_VERSION,
            parser_revision: PARSER_REVISION.to_string(),
            identity,
            content,
            language,
            walked: walked.clone(),
        };
        let bytes = postcard::to_stdvec(&artifact).context("serializing parse cache")?;
        let temp = path.with_extension(format!("tmp-{}", std::process::id()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp)
                .with_context(|| format!("creating {}", temp.display()))?;
            file.write_all(&bytes)
                .with_context(|| format!("writing {}", temp.display()))?;
            fs::rename(&temp, &path).with_context(|| format!("installing {}", path.display()))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }

    fn path_for(&self, key: u64) -> PathBuf {
        self.root.join(CACHE_DIR).join(format!("{key:016x}.bin"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::ir::Language;

    fn cache_fixture() -> (
        tempfile::TempDir,
        ParseCache,
        crate::spine::cache::SourceFingerprint,
        u64,
    ) {
        let root = tempfile::tempdir().unwrap();
        let cache = ParseCache::new(root.path());
        let path = root.path().join("a.py");
        let stamp =
            crate::spine::cache::SourceFingerprint::new(&path, Language::Python, b"x = 1\n");
        let key = stamp.cache_key(SCHEMA_VERSION);
        (root, cache, stamp, key)
    }

    #[test]
    fn corrupt_or_stale_artifacts_are_misses() {
        let (root, cache, stamp, key) = cache_fixture();
        fs::create_dir_all(root.path().join(CACHE_DIR)).unwrap();
        fs::write(cache.path_for(key), b"not postcard").unwrap();
        assert!(cache
            .load(stamp.identity, stamp.content, Language::Python, key)
            .is_none());
    }

    #[test]
    fn persist_replaces_an_invalid_artifact_after_a_miss() {
        let (root, cache, stamp, key) = cache_fixture();
        fs::create_dir_all(root.path().join(CACHE_DIR)).unwrap();
        fs::write(cache.path_for(key), b"stale").unwrap();

        cache
            .persist(
                stamp.identity,
                stamp.content,
                Language::Python,
                key,
                &Walked::default(),
            )
            .unwrap();

        assert!(cache
            .load(stamp.identity, stamp.content, Language::Python, key)
            .is_some());
    }

    #[test]
    fn round_trip_preserves_walked_facts() {
        let (_root, cache, stamp, key) = cache_fixture();
        let mut walked = Walked::default();
        walked.syntax.lexemes.push(42);
        cache
            .persist(
                stamp.identity,
                stamp.content,
                Language::Python,
                key,
                &walked,
            )
            .unwrap();
        let loaded = cache
            .load(stamp.identity, stamp.content, Language::Python, key)
            .unwrap();
        assert_eq!(loaded.syntax.lexemes, vec![42]);
    }
}

//! Parse facts on disk under `.sensez/parse-cache/`, shared by every scan child
//! of a repository. One entry per file, keyed by the sensez build, the config
//! signature, the path and the content hash; written atomically and evicted
//! oldest-first once the directory passes [`BUDGET_BYTES`].

use super::{ChangeStats, ParseCacheState, ProjectInputs, SourceFile, SourceFingerprint};
use crate::spine::ir::{Language, Walked};
use crate::spine::parser::{ParseBatch, ParsedFile};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Disk budget for one repository's entries; entries the current scan uses are never evicted.
const BUDGET_BYTES: u64 = 1 << 30;

/// Whether the disk cache is on; `SENSEZ_PARSE_CACHE=0` (or `false`/`no`/`off`) turns it off.
pub fn enabled() -> bool {
    std::env::var("SENSEZ_PARSE_CACHE").map_or(true, |value| {
        !matches!(
            value.to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off"
        )
    })
}

#[derive(Serialize, Deserialize)]
struct Entry<W> {
    language: Language,
    lines: u32,
    is_generated: bool,
    runs_as_program: bool,
    walked: W,
}

struct DiskCache {
    dir: PathBuf,
    salt: String,
}

/// Restore every file with a valid entry, parse the rest and store their entries.
pub fn parse(
    root: &Path,
    project: &ProjectInputs,
    config_signature: u64,
) -> (ParseBatch, ChangeStats) {
    let cache = DiskCache::new(root, config_signature);
    let mut state = ParseCacheState::default();
    let restored: Vec<ParsedFile> = project
        .sources
        .par_iter()
        .filter_map(|source| cache.read(source))
        .collect();
    for file in &restored {
        state.insert(file);
    }
    drop(restored);
    let (batch, stats) =
        crate::spine::parser::parse_sources_incremental(&project.sources, &mut state);
    let misses: Vec<&ParsedFile> = batch
        .files
        .iter()
        .filter(|file| !state.contains(&file.path, file.fingerprint.content))
        .collect();
    if misses.is_empty() || crate::dotdir::ensure(root, Some("parse-cache")).is_err() {
        return (batch, stats);
    }
    let written = misses
        .par_iter()
        .filter(|file| cache.write(file).is_ok())
        .count();
    if written > 0 {
        cache.evict(&project.sources, BUDGET_BYTES);
    }
    (batch, stats)
}

impl DiskCache {
    fn new(root: &Path, config_signature: u64) -> Self {
        DiskCache {
            dir: root.join(".sensez").join("parse-cache"),
            salt: format!(
                "{}:{}:{config_signature}",
                env!("CARGO_PKG_VERSION"),
                build_id()
            ),
        }
    }

    fn entry_path(&self, path: &Path, content_hash: u64) -> PathBuf {
        let key = crate::fingerprints::hash_parts(&[
            &self.salt,
            &path.to_string_lossy(),
            &content_hash.to_string(),
        ]);
        self.dir
            .join(format!("{:02x}", key >> 56))
            .join(format!("{key:016x}.bin"))
    }

    /// The cached facts for `source`, or `None` when absent, truncated or corrupt.
    fn read(&self, source: &SourceFile) -> Option<ParsedFile> {
        let bytes = std::fs::read(self.entry_path(&source.path, source.content_hash)).ok()?;
        let (checksum, payload) = bytes.split_at_checked(8)?;
        if u64::from_le_bytes(checksum.try_into().ok()?) != crate::fingerprints::hash_bytes(payload)
        {
            return None;
        }
        let entry: Entry<Walked> = postcard::from_bytes(payload).ok()?;
        Some(ParsedFile {
            path: source.path.clone(),
            language: entry.language,
            lines: entry.lines,
            fingerprint: SourceFingerprint::with_content_hash(
                &source.path,
                entry.language,
                source.content_hash,
            ),
            walked: Arc::new(entry.walked),
            is_generated: entry.is_generated,
            runs_as_program: entry.runs_as_program,
        })
    }

    /// Write one entry through a uniquely named temp file, so concurrent writers never expose a partial entry.
    fn write(&self, file: &ParsedFile) -> anyhow::Result<()> {
        let entry = Entry {
            language: file.language,
            lines: file.lines,
            is_generated: file.is_generated,
            runs_as_program: file.runs_as_program,
            walked: &*file.walked,
        };
        let payload = postcard::to_allocvec(&entry)?;
        let mut bytes = crate::fingerprints::hash_bytes(&payload)
            .to_le_bytes()
            .to_vec();
        bytes.extend_from_slice(&payload);
        let target = self.entry_path(&file.path, file.fingerprint.content);
        let parent = target.parent().unwrap_or(&self.dir);
        std::fs::create_dir_all(parent)?;
        let tmp = parent.join(format!(
            ".{}.{}.{:?}.tmp",
            target.file_name().unwrap_or_default().to_string_lossy(),
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &target)?;
        Ok(())
    }

    /// Delete the oldest entries the current scan does not use until the directory fits the budget.
    fn evict(&self, sources: &[SourceFile], budget: u64) {
        let current: HashSet<PathBuf> = sources
            .iter()
            .map(|source| self.entry_path(&source.path, source.content_hash))
            .collect();
        let mut entries: Vec<(std::time::SystemTime, u64, PathBuf)> = entry_files(&self.dir);
        let mut total: u64 = entries.iter().map(|(_, len, _)| len).sum();
        if total <= budget {
            return;
        }
        entries.sort();
        for (_, len, path) in entries {
            if total <= budget {
                break;
            }
            if !current.contains(&path) && std::fs::remove_file(&path).is_ok() {
                total -= len;
            }
        }
    }
}

fn entry_files(dir: &Path) -> Vec<(std::time::SystemTime, u64, PathBuf)> {
    let shards = std::fs::read_dir(dir).into_iter().flatten().flatten();
    shards
        .flat_map(|shard| {
            std::fs::read_dir(shard.path())
                .into_iter()
                .flatten()
                .flatten()
        })
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            Some((meta.modified().ok()?, meta.len(), entry.path()))
        })
        .collect()
}

/// Size and modification time of the running binary, so a rebuild with the same version still misses.
fn build_id() -> String {
    std::env::current_exe()
        .and_then(std::fs::metadata)
        .map(|meta| {
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |since| since.as_nanos());
            format!("{}:{modified}", meta.len())
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "disk_tests.rs"]
mod tests;

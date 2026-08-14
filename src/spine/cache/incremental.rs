use super::{SourceFile, SourceFingerprint};
use crate::source_state::SourceManifest;
use crate::spine::ir::{Language, Walked};
use crate::spine::parser::ParsedFile;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const CACHE_REL: &str = ".sensez/parse-v2.bin";
const SCHEMA: u32 = 1;
const REVISION: &str = concat!(env!("CARGO_PKG_VERSION"), "-walked-v1");
const MAX_DECOMPRESSED_BYTES: u64 = 32_000_000;
const MIN_SOURCE_COVERAGE_PERCENT: usize = 10;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeStats {
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
    pub unchanged: usize,
    pub reusable: usize,
    pub total_bytes: usize,
    pub reusable_bytes: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ManifestEntry {
    path: PathBuf,
    content_hash: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedFile {
    path: PathBuf,
    content_hash: u64,
    source_bytes: usize,
    language: Language,
    lines: u32,
    walked: Walked,
}

impl CachedFile {
    fn restore(mut self, source: &SourceFile, file_id: u32) -> ParsedFile {
        for span in &mut self.walked.syntax.spans {
            span.file_id = file_id;
        }
        ParsedFile {
            path: source.path.clone(),
            language: self.language,
            lines: self.lines,
            fingerprint: SourceFingerprint::new(&source.path, self.language, &source.bytes),
            walked: self.walked,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Artifact {
    schema: u32,
    revision: String,
    manifest: Vec<ManifestEntry>,
    files: Vec<CachedFile>,
}

#[derive(serde::Serialize)]
struct ArtifactRef<'a> {
    schema: u32,
    revision: &'a str,
    manifest: &'a [ManifestEntry],
    files: &'a [CachedFile],
}

#[derive(Default)]
pub struct ParseCacheState {
    manifest: SourceManifest,
    files: HashMap<(PathBuf, u64), CachedFile>,
}

impl ParseCacheState {
    pub fn changes(&self, sources: &[SourceFile]) -> ChangeStats {
        let current = SourceManifest::from_hashes(
            sources
                .iter()
                .map(|source| (source.path.clone(), source.content_hash)),
        );
        let changes = self.manifest.changes(&current);
        let mut stats = ChangeStats::default();
        for source in sources {
            stats.total_bytes += source.bytes.len();
            if self
                .files
                .contains_key(&(source.path.clone(), source.content_hash))
            {
                stats.reusable += 1;
                stats.reusable_bytes += source.bytes.len();
            }
        }
        stats.added = changes.added.len();
        stats.modified = changes.modified.len();
        stats.deleted = changes.deleted.len();
        stats.unchanged = changes.unchanged.len();
        stats
    }

    pub fn restore(&mut self, source: &SourceFile, file_id: u32) -> Option<ParsedFile> {
        self.files
            .remove(&(source.path.clone(), source.content_hash))
            .map(|cached| cached.restore(source, file_id))
    }
}

#[derive(Clone)]
pub struct ParseCache {
    path: PathBuf,
}

pub(crate) struct ParseWriteInput {
    manifest: Vec<ManifestEntry>,
    files: Vec<CachedFile>,
}

pub(super) struct PreparedParse {
    bytes: Vec<u8>,
}

impl ParseCache {
    pub fn new(root: &Path) -> Self {
        let _ = fs::remove_file(root.join(".sensez/analysis-v1.bin"));
        let _ = fs::remove_dir_all(root.join(".sensez/parse-v1"));
        let path = root.join(CACHE_REL);
        super::budget::remove_oversized(&path, super::budget::TOTAL_BYTES);
        Self { path }
    }

    pub fn load(&self) -> ParseCacheState {
        self.try_load().unwrap_or_default()
    }

    pub(super) fn path_key(&self) -> PathBuf {
        self.path.clone()
    }

    fn try_load(&self) -> Option<ParseCacheState> {
        let bytes = fs::read(&self.path).ok()?;
        if bytes.len() > super::budget::TOTAL_BYTES {
            return None;
        }
        let mut decoded = Vec::new();
        GzDecoder::new(bytes.as_slice())
            .take(MAX_DECOMPRESSED_BYTES)
            .read_to_end(&mut decoded)
            .ok()?;
        let artifact: Artifact = postcard::from_bytes(&decoded).ok()?;
        if artifact.schema != SCHEMA || artifact.revision != REVISION {
            return None;
        }
        Some(ParseCacheState {
            manifest: SourceManifest::from_hashes(
                artifact
                    .manifest
                    .into_iter()
                    .map(|entry| (entry.path, entry.content_hash)),
            ),
            files: artifact
                .files
                .into_iter()
                .map(|file| ((file.path.clone(), file.content_hash), file))
                .collect(),
        })
    }

    pub(crate) fn capture(sources: &[SourceFile], parsed: Vec<ParsedFile>) -> ParseWriteInput {
        let sizes: HashMap<_, _> = sources
            .iter()
            .map(|source| (source.path.clone(), source.bytes.len()))
            .collect();
        ParseWriteInput {
            manifest: sources
                .iter()
                .map(|source| ManifestEntry {
                    path: source.path.clone(),
                    content_hash: source.content_hash,
                })
                .collect(),
            files: parsed
                .into_iter()
                .map(|file| CachedFile {
                    source_bytes: sizes.get(&file.path).copied().unwrap_or_default(),
                    path: file.path,
                    content_hash: file.fingerprint.content,
                    language: file.language,
                    lines: file.lines,
                    walked: file.walked,
                })
                .collect(),
        }
    }

    pub fn worth_capturing(sources: &[SourceFile]) -> bool {
        let total: usize = sources.iter().map(|source| source.bytes.len()).sum();
        if total == 0 {
            return false;
        }
        let mut sizes: Vec<_> = sources.iter().map(|source| source.bytes.len()).collect();
        sizes.sort_unstable_by_key(|size| std::cmp::Reverse(*size));
        let mut retained = 0_usize;
        for size in sizes {
            if retained > 0 && retained.saturating_add(size) > super::budget::TOTAL_BYTES {
                break;
            }
            retained = retained.saturating_add(size);
        }
        retained.saturating_mul(100) / total >= MIN_SOURCE_COVERAGE_PERCENT
    }

    pub(super) fn prepare(
        mut input: ParseWriteInput,
        byte_limit: usize,
    ) -> Result<Option<PreparedParse>> {
        input
            .files
            .sort_by_key(|file| std::cmp::Reverse(file.source_bytes));
        // Bound cold-build CPU as well as disk bytes. The largest source files
        // dominate parse time, so retaining a source prefix no larger than the
        // disk budget gives useful reuse without serializing the repository.
        let keep = admitted_prefix(&input.files, byte_limit);
        encode_to_limit(&input, keep, byte_limit)
    }

    pub(super) fn write(&self, prepared: PreparedParse) -> Result<()> {
        super::storage::atomic_write(
            &self.path,
            &prepared.bytes,
            "persisting incremental parse cache",
        )
    }

    pub(super) fn remove(&self) {
        let _ = fs::remove_file(&self.path);
    }

    pub(super) fn len(&self) -> usize {
        fs::metadata(&self.path)
            .map(|metadata| metadata.len() as usize)
            .unwrap_or_default()
    }
}

fn admitted_prefix(files: &[CachedFile], source_limit: usize) -> usize {
    let mut source_bytes: usize = 0;
    let count = files
        .iter()
        .take_while(|file| {
            let next = source_bytes.saturating_add(file.source_bytes);
            if next > source_limit {
                return false;
            }
            source_bytes = next;
            true
        })
        .count();
    count.max(usize::from(!files.is_empty()))
}

fn encode_to_limit(
    input: &ParseWriteInput,
    initial_keep: usize,
    byte_limit: usize,
) -> Result<Option<PreparedParse>> {
    let mut keep = initial_keep;
    loop {
        let bytes = encode(&input.manifest, &input.files[..keep])?;
        if bytes.len() <= byte_limit {
            return Ok(Some(PreparedParse { bytes }));
        }
        if keep == 0 {
            return Ok(None);
        }
        let next = keep.saturating_mul(byte_limit) / bytes.len();
        keep = next.min(keep - 1);
    }
}

fn encode(manifest: &[ManifestEntry], files: &[CachedFile]) -> Result<Vec<u8>> {
    let artifact = ArtifactRef {
        schema: SCHEMA,
        revision: REVISION,
        manifest,
        files,
    };
    let serialized = postcard::to_allocvec(&artifact).context("serializing parse cache")?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&serialized)?;
    Ok(encoder.finish()?)
}

#[cfg(test)]
#[path = "incremental_tests.rs"]
mod tests;

use super::{SourceFile, SourceFingerprint};
use crate::source_state::SourceManifest;
use crate::spine::ir::{Language, Walked};
use crate::spine::parser::ParsedFile;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChangeStats {
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
    pub unchanged: usize,
    pub reusable: usize,
    pub total_bytes: usize,
    pub reusable_bytes: usize,
    pub changed_paths: Vec<PathBuf>,
}

struct CachedFile {
    path: PathBuf,
    content_hash: u64,
    language: Language,
    lines: u32,
    walked: Arc<Walked>,
    uniform_file_id: Option<u32>,
}

impl CachedFile {
    fn restore(&self, source: &SourceFile, file_id: u32) -> ParsedFile {
        let walked =
            if self.walked.syntax.spans.is_empty() || self.uniform_file_id == Some(file_id) {
                Arc::clone(&self.walked)
            } else {
                let mut fresh = (*self.walked).clone();
                for span in &mut fresh.syntax.spans {
                    span.file_id = file_id;
                }
                Arc::new(fresh)
            };
        ParsedFile {
            path: source.path.clone(),
            language: self.language,
            lines: self.lines,
            // A cache hit means `self.content_hash == source.content_hash` by
            // definition, so reuse the stored hash — no third byte-scan.
            fingerprint: SourceFingerprint::with_content_hash(
                &source.path,
                self.language,
                self.content_hash,
            ),
            walked,
        }
    }

    fn from_parsed(file: &ParsedFile) -> Self {
        let first = file.walked.syntax.spans.first().map(|span| span.file_id);
        let uniform = if file
            .walked
            .syntax
            .spans
            .iter()
            .all(|span| Some(span.file_id) == first)
        {
            first
        } else {
            None
        };
        CachedFile {
            path: file.path.clone(),
            content_hash: file.fingerprint.content,
            language: file.language,
            lines: file.lines,
            walked: Arc::clone(&file.walked),
            uniform_file_id: uniform,
        }
    }
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
        stats.changed_paths = changes
            .added
            .iter()
            .chain(&changes.modified)
            .chain(&changes.deleted)
            .cloned()
            .collect();
        stats
    }

    pub fn restore(&mut self, source: &SourceFile, file_id: u32) -> Option<ParsedFile> {
        self.files
            .get(&(source.path.clone(), source.content_hash))
            .map(|cached| cached.restore(source, file_id))
    }

    pub fn replace(&mut self, sources: &[SourceFile], parsed: &[ParsedFile]) {
        self.manifest = SourceManifest::from_hashes(
            sources
                .iter()
                .map(|source| (source.path.clone(), source.content_hash)),
        );
        self.files = parsed
            .iter()
            .map(|file| {
                let cached = CachedFile::from_parsed(file);
                ((cached.path.clone(), cached.content_hash), cached)
            })
            .collect();
    }
}

#[cfg(test)]
#[path = "incremental_tests.rs"]
mod tests;

use super::parsed_file::ParsedFile;
use super::timing;
use super::tree::parse_source_with_parser;
use crate::profiles::registry;
use crate::spine::ir::FileId;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[cfg(test)]
pub fn parse_file(path: &Path, file_id: FileId) -> Result<ParsedFile> {
    parse_file_with_parser(path, file_id, &mut tree_sitter::Parser::new())
}

pub(super) fn parse_file_with_parser(
    path: &Path,
    file_id: FileId,
    parser: &mut tree_sitter::Parser,
) -> Result<ParsedFile> {
    let read_started = timing::enabled().then(std::time::Instant::now);
    let src = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if let Some(started) = read_started {
        timing::record_read(started.elapsed());
    }
    let hash_started = timing::enabled().then(std::time::Instant::now);
    let content_hash = crate::fingerprints::hash_bytes(&src);
    if let Some(started) = hash_started {
        timing::record_hash(started.elapsed());
    }
    parse_loaded_source(
        &crate::spine::cache::SourceFile {
            path: path.to_path_buf(),
            content_hash,
            bytes: src,
        },
        file_id,
        parser,
    )
}

pub(super) fn parse_loaded_source(
    source: &crate::spine::cache::SourceFile,
    file_id: FileId,
    parser: &mut tree_sitter::Parser,
) -> Result<ParsedFile> {
    let path = &source.path;
    let src = &source.bytes;
    let profile = registry::parse_for_path(path)
        .ok_or_else(|| anyhow!("no language profile for {}", path.display()))?;
    let module_name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let fingerprint = crate::spine::cache::SourceFingerprint::with_content_hash(
        path,
        profile.info().language,
        source.content_hash,
    );
    let is_generated = profile.is_generated(path, src);
    let walked = parse_source_with_parser(src, file_id, &module_name, profile, parser)
        .with_context(|| {
            if is_generated {
                format!("generated file skipped: {}", path.display())
            } else {
                format!("parsing {}", path.display())
            }
        })?;
    let lines = if src.is_empty() {
        0
    } else {
        u32::try_from(src.iter().filter(|&&b| b == b'\n').count() + 1).unwrap_or(u32::MAX)
    };
    Ok(ParsedFile {
        path: path.to_path_buf(),
        language: profile.info().language,
        lines,
        fingerprint,
        walked: Arc::new(walked),
        is_generated,
        runs_as_program: profile.runs_as_program(src),
    })
}

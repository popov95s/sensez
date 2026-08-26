pub use crate::spine::ir::tokens;
pub(crate) mod timing;
#[allow(unused_imports)]
pub use crate::spine::ir::tokens::{StructuralToken, TokenSpan};
pub use crate::spine::ir::{
    ClassProperty, FunctionUnit, ImportContext, ImportPhase, SymbolKind, Walked,
};

use crate::profiles::{registry, ParseProfile};
use crate::report::{ScanIssue, ScanStage};
pub use crate::spine::cache::SourceFingerprint;
use crate::spine::ir::Language;
use anyhow::{anyhow, Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct ParsedFile {
    pub path: PathBuf,
    pub language: Language,
    pub lines: u32,
    /// Identity + content stamps for the incremental cache and semantic
    /// duplication keys.
    pub fingerprint: SourceFingerprint,
    /// Shared so the incremental cache can hand out copies of an unchanged file
    /// without deep-cloning the whole token stream. The single writer of a
    /// file's `Walked` (the parser) produces an `Arc`; analyzers read through
    /// it, and the one mutating caller uses [`Arc::make_mut`].
    pub walked: Arc<Walked>,
}

#[derive(Debug, Default)]
pub struct ParseBatch {
    pub files: Vec<ParsedFile>,
    pub issues: Vec<ScanIssue>,
}

pub fn parse_files(files: &[PathBuf]) -> ParseBatch {
    let outcomes: Vec<_> = files
        .par_iter()
        .enumerate()
        .map_init(
            tree_sitter::Parser::new,
            |parser, (i, path)| match parse_file_with_parser(path, i as u32, parser) {
                Ok(parsed) => Ok(parsed),
                Err(err) => Err(ScanIssue {
                    stage: ScanStage::Parse,
                    file: Some(path.clone()),
                    message: format!("{err:#}"),
                }),
            },
        )
        .collect();

    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    for outcome in outcomes {
        match outcome {
            Ok(file) => parsed.push(file),
            Err(issue) => issues.push(issue),
        }
    }
    ParseBatch {
        files: parsed,
        issues,
    }
}

pub fn parse_sources(files: &[crate::spine::cache::SourceFile]) -> ParseBatch {
    let outcomes: Vec<_> = files
        .par_iter()
        .enumerate()
        .map_init(tree_sitter::Parser::new, |parser, (i, source)| {
            parse_loaded_source(source, i as u32, parser).map_err(|err| ScanIssue {
                stage: ScanStage::Parse,
                file: Some(source.path.clone()),
                message: format!("{err:#}"),
            })
        })
        .collect();

    let mut parsed = Vec::new();
    let mut issues = Vec::new();
    for outcome in outcomes {
        match outcome {
            Ok(file) => parsed.push(file),
            Err(issue) => issues.push(issue),
        }
    }
    ParseBatch {
        files: parsed,
        issues,
    }
}

pub fn parse_sources_incremental(
    files: &[crate::spine::cache::SourceFile],
    cache: &mut crate::spine::cache::ParseCacheState,
) -> (ParseBatch, crate::spine::cache::ChangeStats) {
    let stats = cache.changes(files);
    let mut outcomes: Vec<Option<Result<ParsedFile>>> = (0..files.len()).map(|_| None).collect();
    for (index, source) in files.iter().enumerate() {
        if let Some(parsed) = cache.restore(source, index as u32) {
            outcomes[index] = Some(Ok(parsed));
        }
    }
    let misses: Vec<_> = files
        .par_iter()
        .enumerate()
        .filter(|(index, _)| outcomes[*index].is_none())
        .map_init(tree_sitter::Parser::new, |parser, (index, source)| {
            (index, parse_loaded_source(source, index as u32, parser))
        })
        .collect();
    for (index, outcome) in misses {
        outcomes[index] = Some(outcome);
    }

    let mut batch = ParseBatch::default();
    for (source, outcome) in files.iter().zip(outcomes) {
        match outcome {
            Some(Ok(file)) => batch.files.push(file),
            Some(Err(err)) => batch.issues.push(ScanIssue {
                stage: ScanStage::Parse,
                file: Some(source.path.clone()),
                message: format!("{err:#}"),
            }),
            None => unreachable!("every source is restored or parsed"),
        }
    }
    (batch, stats)
}

#[cfg(test)]
pub fn parse_file(path: &Path, file_id: u32) -> Result<ParsedFile> {
    parse_file_with_parser(path, file_id, &mut tree_sitter::Parser::new())
}

fn parse_file_with_parser(
    path: &Path,
    file_id: u32,
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

fn parse_loaded_source(
    source: &crate::spine::cache::SourceFile,
    file_id: u32,
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
    let walked = parse_source_with_parser(src, file_id, &module_name, profile, parser)
        .with_context(|| format!("parsing {}", path.display()))?;
    let lines = if src.is_empty() {
        0
    } else {
        (src.iter().filter(|&&b| b == b'\n').count() + 1) as u32
    };
    Ok(ParsedFile {
        path: path.to_path_buf(),
        language: profile.info().language,
        lines,
        fingerprint,
        walked: Arc::new(walked),
    })
}

const MAX_TREE_DEPTH: usize = 512;

#[cfg(test)]
pub fn parse_source(
    src: &[u8],
    file_id: u32,
    module_name: &str,
    profile: &dyn ParseProfile,
) -> Result<Walked> {
    let mut parser = tree_sitter::Parser::new();
    parse_source_with_parser(src, file_id, module_name, profile, &mut parser)
}

fn parse_source_with_parser(
    src: &[u8],
    file_id: u32,
    module_name: &str,
    profile: &dyn ParseProfile,
    parser: &mut tree_sitter::Parser,
) -> Result<Walked> {
    parser
        .set_language(&profile.ts_language())
        .context("incompatible tree-sitter grammar ABI")?;
    let tree_started = timing::enabled().then(std::time::Instant::now);
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| anyhow!("tree-sitter returned no tree"))?;
    if let Some(started) = tree_started {
        timing::record_tree(started.elapsed());
    }
    let depth_started = timing::enabled().then(std::time::Instant::now);
    if tree_depth(tree.root_node(), MAX_TREE_DEPTH) > MAX_TREE_DEPTH {
        return Err(anyhow!(
            "syntax tree deeper than {MAX_TREE_DEPTH} levels; skipping (DoS guard)"
        ));
    }
    if let Some(started) = depth_started {
        timing::record_depth(started.elapsed());
    }
    let walk_started = timing::enabled().then(std::time::Instant::now);
    let walked = profile.walk(tree.root_node(), src, file_id, module_name);
    if let Some(started) = walk_started {
        timing::record_walk(started.elapsed());
    }
    Ok(walked)
}

fn tree_depth(root: tree_sitter::Node, limit: usize) -> usize {
    let mut cursor = root.walk();
    let mut depth = TreeDepth::default();
    loop {
        if cursor.goto_first_child() {
            if depth.descend() > limit {
                return depth.max();
            }
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return depth.max();
            }
            depth.ascend();
        }
    }
}

#[derive(Default)]
struct TreeDepth {
    current: usize,
    max: usize,
}

impl TreeDepth {
    fn descend(&mut self) -> usize {
        self.current += 1;
        self.max = self.max.max(self.current);
        self.max
    }

    fn ascend(&mut self) {
        self.current = self.current.saturating_sub(1);
    }

    fn max(&self) -> usize {
        self.max
    }
}

#[cfg(test)]
mod tests;

//! Routes each file to its [`ParseProfile`](crate::profiles::ParseProfile)
//! by extension and applies the shared safety gates (grammar ABI check, tree
//! depth guard). The language-neutral output types live in [`crate::spine::ir`] and
//! are re-exported here for convenience; all grammar-specific walking lives
//! under `crate::profiles`.

pub use crate::spine::ir::tokens;
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

/// A fully parsed source file (any supported language): file identity plus the
/// language-neutral walk output. `walked` is the single source of truth for
/// everything extracted from the syntax tree — never mirrored field-by-field.
#[derive(Debug)]
pub struct ParsedFile {
    pub path: PathBuf,
    /// The language this file was parsed as (drives graph/dead-code dispatch).
    pub language: Language,
    /// Source line count (the size denominator for scan-throughput health).
    pub lines: u32,
    /// Stable source identity plus content hash used by incremental caches.
    #[allow(dead_code)]
    pub fingerprint: SourceFingerprint,
    /// The walk output ([`Walked`]) for this file.
    pub walked: Walked,
}

/// Parsed files plus any concrete per-file failures.
#[derive(Debug, Default)]
pub struct ParseBatch {
    pub files: Vec<ParsedFile>,
    pub issues: Vec<ScanIssue>,
}

/// Parse many files in parallel, preserving concrete failures as diagnostics.
#[allow(dead_code)]
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

/// Parse buffers already read while computing the project fingerprint.
#[allow(dead_code)]
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

/// Reuse path+content-matched walk outputs and parse only cache misses. The
/// returned change set describes the complete manifest, even when the bounded
/// cache could not retain every unchanged file.
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

/// Parse a single file from disk, routed to its language profile by extension.
#[allow(dead_code)]
pub fn parse_file(path: &Path, file_id: u32) -> Result<ParsedFile> {
    parse_file_with_parser(path, file_id, &mut tree_sitter::Parser::new())
}

fn parse_file_with_parser(
    path: &Path,
    file_id: u32,
    parser: &mut tree_sitter::Parser,
) -> Result<ParsedFile> {
    let src = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    parse_loaded_source(
        &crate::spine::cache::SourceFile {
            path: path.to_path_buf(),
            content_hash: crate::fingerprints::hash_bytes(&src),
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
    let fingerprint =
        crate::spine::cache::SourceFingerprint::new(path, profile.info().language, src);
    let walked = parse_source_with_parser(src, file_id, &module_name, profile, parser)
        .with_context(|| format!("parsing {}", path.display()))?;
    // Lines = newline count + 1 for a trailing partial line; 0 for an empty file.
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
        walked,
    })
}

/// Deepest syntax tree the recursive walkers will accept. Real code rarely
/// nests past ~50; pathological/adversarial input (e.g. `((((…))))` × 100k)
/// would otherwise overflow the stack of every recursive consumer (walk,
/// unit analysis, type hints). One gate here protects them all.
const MAX_TREE_DEPTH: usize = 512;

/// Parse source bytes with the given language profile (no filesystem access).
#[allow(dead_code)]
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
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| anyhow!("tree-sitter returned no tree"))?;
    if tree_depth(tree.root_node(), MAX_TREE_DEPTH) > MAX_TREE_DEPTH {
        return Err(anyhow!(
            "syntax tree deeper than {MAX_TREE_DEPTH} levels; skipping (DoS guard)"
        ));
    }
    Ok(profile.walk(tree.root_node(), src, file_id, module_name))
}

/// Iterative (cursor-based, no recursion) tree depth, capped at `limit + 1`
/// so adversarial input can't make the measurement itself expensive.
///
/// Returns the maximum depth of any node. **The root counts as depth 1** —
/// `tree_depth(leaf, 100)` returns `1` for a single-node tree and `2` for a
/// flat `program` with one child, matching the convention tree-sitter uses
/// for `Node::descendant_count`/`Tree::root_node`. Callers comparing against
/// `MAX_TREE_DEPTH` should treat the root as the first level.
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

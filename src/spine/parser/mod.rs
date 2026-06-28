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
pub fn parse_files(files: &[PathBuf]) -> ParseBatch {
    let outcomes: Vec<_> = files
        .par_iter()
        .enumerate()
        .map(|(i, path)| match parse_file(path, i as u32) {
            Ok(parsed) => Ok(parsed),
            Err(err) => Err(ScanIssue {
                stage: ScanStage::Parse,
                file: Some(path.clone()),
                message: format!("{err:#}"),
            }),
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

/// Parse a single file from disk, routed to its language profile by extension.
pub fn parse_file(path: &Path, file_id: u32) -> Result<ParsedFile> {
    let profile = registry::parse_for_path(path)
        .ok_or_else(|| anyhow!("no language profile for {}", path.display()))?;
    let src = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let module_name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let walked = parse_source(&src, file_id, &module_name, profile)
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
        walked,
    })
}

/// Deepest syntax tree the recursive walkers will accept. Real code rarely
/// nests past ~50; pathological/adversarial input (e.g. `((((…))))` × 100k)
/// would otherwise overflow the stack of every recursive consumer (walk,
/// unit analysis, type hints). One gate here protects them all.
const MAX_TREE_DEPTH: usize = 512;

/// Parse source bytes with the given language profile (no filesystem access).
pub fn parse_source(
    src: &[u8],
    file_id: u32,
    module_name: &str,
    profile: &dyn ParseProfile,
) -> Result<Walked> {
    let mut parser = tree_sitter::Parser::new();
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
mod depth_tests {
    use super::*;
    use crate::profiles::registry;

    #[test]
    fn pathological_nesting_is_rejected_not_crashed() {
        let profile = registry::parse_for_path(Path::new("x.py")).unwrap();
        // 100k-deep parenthesized expression would overflow recursive walkers.
        let src = format!("x = {}1{}", "(".repeat(100_000), ")".repeat(100_000));
        let result = parse_source(src.as_bytes(), 0, "x", profile);
        assert!(result.is_err(), "deep tree must be rejected, not walked");

        // Sane code is untouched.
        let ok = parse_source(b"def f():\n    return (1 + 2)\n", 0, "x", profile);
        assert!(ok.is_ok());
    }

    /// The depth gate is exact: a tree at MAX_TREE_DEPTH parses, one level
    /// deeper is rejected. The fixed overhead between paren count and tree
    /// depth is measured rather than assumed, so a grammar bump that changes
    /// node nesting cannot silently shift the boundary under this test.
    #[test]
    fn depth_gate_boundary_is_exact() {
        let profile = registry::parse_for_path(Path::new("x.py")).unwrap();
        let src_with = |parens: usize| format!("x = {}1{}", "(".repeat(parens), ")".repeat(parens));
        let depth_of = |parens: usize| {
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&profile.ts_language()).unwrap();
            let tree = parser.parse(src_with(parens).as_bytes(), None).unwrap();
            tree_depth(tree.root_node(), usize::MAX)
        };
        // depth(k) = k + overhead (each paren nests exactly one level).
        let overhead = depth_of(10) - 10;
        let at_limit = MAX_TREE_DEPTH - overhead;

        assert_eq!(depth_of(at_limit), MAX_TREE_DEPTH, "calibration");
        assert!(
            parse_source(src_with(at_limit).as_bytes(), 0, "x", profile).is_ok(),
            "depth == MAX_TREE_DEPTH (512) must be accepted"
        );
        assert!(
            parse_source(src_with(at_limit + 1).as_bytes(), 0, "x", profile).is_err(),
            "depth == MAX_TREE_DEPTH + 1 (513) must be rejected"
        );
    }
}

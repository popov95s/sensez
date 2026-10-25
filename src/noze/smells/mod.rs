//! Pillar 5: design-smell detection.
//!
//! File-local smells (complexity, size, structure, cohesion, type-assisted
//! coupling) read only a [`ParsedFile`] and run in parallel. Cross-file smells
//! (data clumps) aggregate over every function, and graph-metric smells
//! (shotgun-surgery hazard, god module, instability) read the dependency graph.

mod clumps;
mod cohesion;
mod complexity;
mod coupling;
mod exclusive_paths;
mod external_vocab;
mod graphy;
mod inherit;
mod lines;
mod magic_string;
mod mutation;
mod performance;
mod review_risks;
mod size;
mod structural;
mod typing;
mod union_find;

#[cfg(test)]
mod cohesion_tests;
#[cfg(test)]
mod coupling_tests;
#[cfg(test)]
mod defensive_code_tests;
#[cfg(test)]
mod god_module_tests;
#[cfg(test)]
mod graph_tests;
#[cfg(test)]
mod review_risks_tests;
#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod structural_tests;
#[cfg(test)]
mod test_oracle_tests;
#[cfg(test)]
pub(crate) mod test_support;

use crate::config::smells::{SmellConfig, Smells};
use crate::globs::compile_validated;
use crate::noze::source_scope::SourceScope;
use crate::profiles::wording::SmellWording;
use crate::profiles::{registry, TypeVocabularyProfile};
use crate::report::{ActionLevel, Severity, SmellFinding, SmellKind};
use crate::spine::graph::{CodebaseGraph, ModuleNode};
use crate::spine::ir::Language;
use crate::spine::ir::{FunctionUnit, TypeHints};
use crate::spine::parser::ParsedFile;
use rayon::prelude::*;
use rustc_hash::FxHashSet;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Per-file context every smell detector needs: the path (for finding
/// anchors), the language (for type-vocab + structure-target messages), and
/// the type-hint table (for type-assisted smells). Bundling these into one
/// struct trims the per-detector argument list and makes it clear that
/// "function-local" detectors do not, in fact, get access to the graph.
pub(super) struct SmellContext<'a> {
    pub path: &'a Path,
    pub language: Language,
    pub type_hints: &'a TypeHints,
    pub type_vocabulary: &'static dyn TypeVocabularyProfile,
    pub wording: &'static SmellWording,
    pub test_source: bool,
    /// Local names bound by imports from generated modules; chains from them are API navigation.
    pub generated_roots: FxHashSet<&'a str>,
}

impl<'a> SmellContext<'a> {
    pub fn from_file(file: &'a ParsedFile) -> Self {
        Self {
            path: &file.path,
            language: file.language,
            type_hints: &file.walked.units.type_hints,
            type_vocabulary: registry::type_vocabulary(file.language),
            wording: registry::smell_wording(file.language),
            test_source: registry::is_test_source(file.language, &file.path),
            generated_roots: file
                .walked
                .symbols
                .imports
                .iter()
                .filter(|import| {
                    registry::is_generated_import(file.language, &import.target_module)
                })
                .flat_map(|import| import.bindings.iter().map(String::as_str))
                .collect(),
        }
    }
}

/// Detect every enabled smell across the corpus. Each file is analyzed with the
/// knob set resolved for its language ([`SmellConfig::for_language`]).
pub fn detect(
    files: &[ParsedFile],
    graph: &CodebaseGraph,
    cfg: &SmellConfig,
    scope: &SourceScope,
) -> Vec<SmellFinding> {
    if !cfg.enabled {
        return Vec::new();
    }
    let kept: Vec<&ParsedFile> = files.iter().filter(|f| scope.includes(f)).collect();

    let mut out: Vec<SmellFinding> = kept
        .par_iter()
        .flat_map(|f| detect_local(f, cfg.for_language(f.language)))
        .collect();
    let explicit = compile_validated(&cfg.explicit_exclude);
    out.par_extend(
        files
            .par_iter()
            .filter(|file| !scope.includes(file) && !explicit.is_match(&file.path))
            .flat_map(|file| test_oracles(file, cfg.for_language(file.language))),
    );

    let mut cross_file = clumps::detect(&kept, cfg);
    cross_file.extend(graphy::detect(graph, files, cfg, &|node: &ModuleNode| {
        node.source_index
            .and_then(|index| files.get(index))
            .is_some_and(|file| scope.includes(file))
    }));
    apply_rule_actions(&mut cross_file, &kept, cfg);
    out.extend(cross_file);
    out
}

/// The opt-in test rule on a file the ordinary smell scope skips.
fn test_oracles(file: &ParsedFile, settings: &Smells) -> Vec<SmellFinding> {
    if !settings.is_enabled(SmellKind::WeakTestOracle) {
        return Vec::new();
    }
    let ctx = SmellContext::from_file(file);
    let mut findings =
        review_risks::detect_test_oracles(&ctx, &file.walked.units.functions, settings);
    apply_actions(&mut findings, settings);
    fill_spans(file, &mut findings);
    findings
}

/// All smells computable from a single file (no graph, no cross-file data),
/// using the already-resolved per-language knob set. This is the set a future
/// live/in-editor mode would run per buffer.
pub fn detect_local(file: &ParsedFile, cfg: &Smells) -> Vec<SmellFinding> {
    let ctx = SmellContext::from_file(file);
    let metrics: &[FunctionUnit] = &file.walked.units.functions;

    let mut out = Vec::new();
    let classes = file.walked.units.classes.as_slice();
    let usage = coupling::UsageFacts {
        attribute_accesses: &file.walked.usage.attribute_accesses,
    };
    let locals: HashSet<&str> = classes.iter().map(|c| c.name.as_str()).collect();
    complexity::detect(&ctx, metrics, cfg, &mut out);
    size::detect(&ctx, metrics, cfg, classes, &mut out);
    structural::detect(&ctx, metrics, cfg, &mut out);
    cohesion::detect(&ctx, metrics, classes, &mut out);
    review_risks::detect(&ctx, metrics, classes, cfg, &mut out);
    coupling::detect(&ctx, metrics, &usage, &locals, cfg, &mut out);
    inherit::detect(&ctx, classes, cfg, &mut out);
    typing::detect(&ctx, metrics, cfg, &mut out);
    mutation::detect(&ctx, metrics, cfg, &mut out);
    out.extend(performance::detect(&ctx, metrics, cfg));
    out.retain(|f| cfg.is_enabled(f.kind));
    apply_actions(&mut out, cfg);
    fill_spans(file, &mut out);
    out
}

fn apply_actions(findings: &mut [SmellFinding], cfg: &Smells) {
    for finding in findings {
        if let Some(action) = cfg.actions.get(&finding.kind) {
            finding.action = *action;
        }
    }
}

fn apply_rule_actions(findings: &mut [SmellFinding], files: &[&ParsedFile], cfg: &SmellConfig) {
    let by_path: HashMap<_, _> = files
        .iter()
        .map(|file| (file.path.as_path(), file.language))
        .collect();
    for finding in findings {
        if let Some(language) = by_path.get(finding.file.as_path()) {
            apply_actions(std::slice::from_mut(finding), cfg.for_language(*language));
        }
    }
}

/// Fill each file-local finding's `end_line` from the span of the function or
/// class it anchors to (anchor `line` == the unit's `start_line`), enabling
/// body-aware `--diff` scoping. A finding whose anchor matches no unit keeps
/// `end_line == line` (single-line scope).
fn fill_spans(file: &ParsedFile, out: &mut [SmellFinding]) {
    let mut span: HashMap<usize, usize> = HashMap::new();
    for f in &file.walked.units.functions {
        span.insert(f.start_line, f.end_line);
    }
    for c in &file.walked.units.classes {
        span.entry(c.start_line).or_insert(c.end_line);
    }
    for finding in out.iter_mut() {
        if finding.end_line == 0 {
            finding.end_line = span.get(&finding.line).copied().unwrap_or(finding.line);
        }
    }
}

/// Construct a finding (keeps the per-smell call sites terse).
#[allow(clippy::too_many_arguments)]
pub(super) fn make(
    kind: SmellKind,
    message: String,
    file: &Path,
    line: usize,
    symbol: &str,
    severity: Severity,
    metric: u32,
    threshold: u32,
) -> SmellFinding {
    SmellFinding {
        action: ActionLevel::from_severity(severity),
        kind,
        message,
        file: file.to_path_buf(),
        line,
        end_line: 0, // filled per file by `fill_spans` (0 => anchor-line scope)
        lines: Vec::new(),
        symbol: symbol.to_string(),
        severity,
        metric,
        threshold,
        reason: String::new(),
    }
}

#[cfg(test)]
mod comments_tests;
#[cfg(test)]
mod determinism_tests;
#[cfg(test)]
mod discipline_tests;
#[cfg(test)]
mod line_anchor_tests;
#[cfg(test)]
mod loop_frame_tests;
#[cfg(test)]
mod magic_string_tests;
#[cfg(test)]
mod message_chain_tests;
#[cfg(test)]
mod nested_if_tests;
#[cfg(test)]
mod nested_ternary_tests;
#[cfg(test)]
mod performance_tests;
#[cfg(test)]
mod tests;

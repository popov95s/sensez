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
mod graphy;
mod inherit;
mod mutation;
mod performance;
mod size;
mod structural;
mod typing;
mod union_find;

use crate::config::smells::{SmellConfig, Smells};
use crate::globs::build_globset;
use crate::noze::{ActionLevel, Severity, SmellFinding, SmellKind};
use crate::spine::ir::Language;
use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::{FunctionMetrics, TypeHints};
use crate::spine::parser::ParsedFile;
use globset::GlobSet;
use rayon::prelude::*;
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
}

impl<'a> SmellContext<'a> {
    pub fn from_file(file: &'a ParsedFile) -> Self {
        Self {
            path: &file.path,
            language: file.language,
            type_hints: &file.walked.units.type_hints,
        }
    }
}

/// Detect every enabled smell across the corpus. Each file is analyzed with the
/// knob set resolved for its language ([`SmellConfig::for_language`]).
pub fn detect(files: &[ParsedFile], graph: &CodebaseGraph, cfg: &SmellConfig) -> Vec<SmellFinding> {
    if !cfg.enabled {
        return Vec::new();
    }
    let excluded = build_globset(&cfg.exclude).unwrap_or_else(|_| GlobSet::empty());
    let kept: Vec<&ParsedFile> = files
        .iter()
        .filter(|f| !excluded.is_match(&f.path))
        .collect();

    let mut out: Vec<SmellFinding> = kept
        .par_iter()
        .flat_map(|f| detect_local(f, cfg.for_language(f.language)))
        .collect();

    out.extend(clumps::detect(&kept, cfg));
    out.extend(graphy::detect(graph, cfg));
    apply_rule_actions(&mut out, &kept, cfg);
    out
}

/// All smells computable from a single file (no graph, no cross-file data),
/// using the already-resolved per-language knob set. This is the set a future
/// live/in-editor mode would run per buffer.
pub fn detect_local(file: &ParsedFile, cfg: &Smells) -> Vec<SmellFinding> {
    let ctx = SmellContext::from_file(file);
    let metrics: Vec<FunctionMetrics> = file
        .walked
        .units
        .functions
        .iter()
        .map(FunctionMetrics::from)
        .collect();

    let mut out = Vec::new();
    let classes = file.walked.units.classes.as_slice();
    let usage = coupling::UsageFacts {
        attribute_accesses: file.walked.usage.attribute_accesses.clone(),
    };
    let locals: HashSet<&str> = classes.iter().map(|c| c.name.as_str()).collect();
    complexity::detect(&ctx, &metrics, cfg, &mut out);
    size::detect(&ctx, &metrics, cfg, classes, &mut out);
    structural::detect(&ctx, &metrics, cfg, &mut out);
    cohesion::detect(&ctx, &metrics, classes, &mut out);
    coupling::detect(&ctx, &metrics, &usage, &locals, cfg, &mut out);
    inherit::detect(&ctx, classes, cfg, &mut out);
    typing::detect(&ctx, &metrics, cfg, &mut out);
    mutation::detect(&ctx, &metrics, cfg, &mut out);
    performance::detect(&ctx, &metrics, cfg, &mut out);
    // Uniform per-smell on/off: drop any kind disabled for this language.
    if !cfg.disabled.is_empty() {
        out.retain(|f| !cfg.disabled.contains(&f.kind));
    }
    for finding in &mut out {
        if let Some(action) = cfg.actions.get(&finding.kind) {
            finding.action = *action;
        }
    }
    fill_spans(file, &mut out);
    out
}

fn apply_rule_actions(findings: &mut [SmellFinding], files: &[&ParsedFile], cfg: &SmellConfig) {
    let by_path: HashMap<_, _> = files
        .iter()
        .map(|file| (file.path.as_path(), file.language))
        .collect();
    for finding in findings {
        let Some(language) = by_path.get(finding.file.as_path()) else {
            continue;
        };
        if let Some(action) = cfg.for_language(*language).actions.get(&finding.kind) {
            finding.action = *action;
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
        symbol: symbol.to_string(),
        severity,
        metric,
        threshold,
        reason: String::new(),
    }
}

pub(super) fn structure_target(language: Language) -> &'static str {
    match language {
        Language::Python => "a dataclass or model",
        Language::JavaScript | Language::TypeScript => "a typed object or interface",
        Language::Rust => "a struct",
    }
}

pub(super) fn grouped_value_target(language: Language) -> &'static str {
    match language {
        Language::Python => "a NamedTuple or dataclass",
        Language::JavaScript | Language::TypeScript => "a typed object or tuple shape",
        Language::Rust => "a struct",
    }
}

#[cfg(test)]
mod determinism_tests;
#[cfg(test)]
mod discipline_tests;
#[cfg(test)]
mod nested_if_tests;
#[cfg(test)]
mod performance_tests;
#[cfg(test)]
mod tests;

//! Presentation-only report filtering. This never changes scan scope: analyzers
//! run over the full project first, then the rendered/serialized report is
//! narrowed to findings whose source paths match the requested globs.
//!
//! Sits in `reporter` (not its own module) because every other type here is
//! a view over [`crate::report::AnalysisReport`] and the filter is just
//! another one — same data, narrower slice.

use crate::globs::build_globset;
use crate::report::AnalysisReport;
use anyhow::Result;
use globset::GlobSet;
use std::path::{Path, PathBuf};

/// Drop findings from `report` whose source paths do not match the
/// `--output-glob` patterns (`!pattern` excludes). Updates the per-pillar
/// totals and the derived `smell_totals` / `glossary` so the rendered report
/// stays internally consistent.
pub fn apply(report: &mut AnalysisReport, root: &Path, patterns: &[String]) -> Result<()> {
    if patterns.is_empty() {
        return Ok(());
    }
    let filter = OutputPathFilter::new(root, patterns)?;

    report
        .cycles
        .retain(|cycle| cycle.edges.iter().any(|edge| filter.matches(&edge.file)));
    report
        .dead_code
        .retain(|finding| filter.matches(&finding.file));
    report
        .boundaries
        .retain(|violation| filter.matches(&violation.file));
    report.duplication.retain_mut(|class| {
        class.keep_occurrences(|occurrence| !filter.excludes(&occurrence.file))
            && class
                .occurrences
                .iter()
                .any(|occurrence| filter.matches(&occurrence.file))
    });
    report
        .smells
        .retain(|finding| filter.matches(&finding.file));
    report
        .meta
        .issues
        .retain(|issue| issue.file.as_ref().is_none_or(|file| filter.matches(file)));

    report.refresh_totals();
    report.meta.files_skipped = report.meta.issues.len();
    report.meta.glossary = crate::noze::glossary::for_report(report);
    Ok(())
}

struct OutputPathFilter {
    root: PathBuf,
    include: PatternSet,
    exclude: PatternSet,
}

impl OutputPathFilter {
    fn new(root: &Path, patterns: &[String]) -> Result<Self> {
        let mut includes = Vec::new();
        let mut excludes = Vec::new();
        for pattern in patterns {
            match pattern.strip_prefix('!') {
                Some(body) if body.is_empty() || body.starts_with('!') => anyhow::bail!(
                    "--output-glob {pattern:?} can never match: write one leading '!' followed by a glob"
                ),
                Some(body) => excludes.push(body),
                None => includes.push(pattern.as_str()),
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            include: PatternSet::new(&includes)?,
            exclude: PatternSet::new(&excludes)?,
        })
    }

    /// A path matches when no exclude hits and, if includes are given, one of them does.
    fn matches(&self, path: &Path) -> bool {
        (self.include.is_empty() || self.include.hits(&self.root, path)) && !self.excludes(path)
    }

    fn excludes(&self, path: &Path) -> bool {
        self.exclude.hits(&self.root, path)
    }
}

/// Globs plus bare directory names; a path hits when either strategy does.
struct PatternSet {
    globs: GlobSet,
    component_patterns: Vec<String>,
}

impl PatternSet {
    fn new(patterns: &[&str]) -> Result<Self> {
        Ok(Self {
            globs: build_globset(patterns)?,
            component_patterns: patterns
                .iter()
                .filter(|pattern| is_single_literal_component(pattern))
                .map(|pattern| pattern.to_string())
                .collect(),
        })
    }

    fn is_empty(&self) -> bool {
        self.globs.is_empty()
    }

    fn hits(&self, root: &Path, path: &Path) -> bool {
        self.glob_matches(root, path) || self.component_matches(root, path)
    }

    /// Glob match against either the path as-given or its root-relative form.
    fn glob_matches(&self, root: &Path, path: &Path) -> bool {
        if self.globs.is_match(path) {
            return true;
        }
        path.strip_prefix(root)
            .ok()
            .is_some_and(|relative| self.globs.is_match(relative))
    }

    /// Literal path-component match: `packages` matches any path with a `packages` directory.
    fn component_matches(&self, root: &Path, path: &Path) -> bool {
        self.component_patterns.iter().any(|pattern| {
            path_has_component(path, pattern) || relative_has_component(path, root, pattern)
        })
    }
}

fn path_has_component(path: &Path, pattern: &str) -> bool {
    path.components().any(|c| c.as_os_str() == pattern)
}

fn relative_has_component(path: &Path, root: &Path, pattern: &str) -> bool {
    path.strip_prefix(root)
        .ok()
        .is_some_and(|relative| path_has_component(relative, pattern))
}

fn is_single_literal_component(pattern: &str) -> bool {
    !pattern.contains(['*', '?', '[', '/', '\\'])
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod tests;

//! Presentation-only report filtering. This never changes scan scope: analyzers
//! run over the full project first, then the rendered/serialized report is
//! narrowed to findings whose source paths match the requested globs.
//!
//! Sits in `reporter` (not its own module) because every other type here is
//! a view over [`crate::report::AnalysisReport`] and the filter is just
//! another one — same data, narrower slice.

use crate::report::AnalysisReport;
use anyhow::Result;
use globset::{Glob, GlobSet};
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
                Some(body) => excludes.push(exclude_body(pattern, body)?),
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

fn exclude_body<'a>(pattern: &str, body: &'a str) -> Result<&'a str> {
    if body.is_empty() || body.starts_with('!') {
        anyhow::bail!(
            "--output-glob {pattern:?} can never match: write one leading '!' followed by a glob"
        );
    }
    Ok(body)
}

/// Globs plus bare directory names; a path hits when either strategy does.
struct PatternSet {
    globs: GlobSet,
    component_patterns: Vec<String>,
}

impl PatternSet {
    fn new(patterns: &[&str]) -> Result<Self> {
        let mut builder = GlobSet::builder();
        let mut component_patterns = Vec::new();
        for pattern in patterns {
            builder.add(Glob::new(pattern)?);
            if is_single_literal_component(pattern) {
                component_patterns.push(pattern.to_string());
            }
        }
        Ok(Self {
            globs: builder.build()?,
            component_patterns,
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
mod tests {
    use super::*;
    use crate::report::{
        ActionLevel, AnalysisReport, BoundaryViolation, CloneClass, CloneOccurrence, Confidence,
        DeadCodeFinding, ReportMeta, ReportMode, Severity, SmellFinding, SmellKind,
    };
    use crate::spine::ir::SymbolKind;

    #[test]
    fn filters_findings_by_root_relative_glob_after_full_analysis() {
        let root = Path::new("/repo");
        let mut report = AnalysisReport {
            dead_code: vec![dead("/repo/packages/a.py"), dead("/repo/services/b.py")],
            smells: vec![smell("/repo/packages/a.py"), smell("/repo/services/b.py")],
            boundaries: vec![
                boundary("/repo/packages/a.py"),
                boundary("/repo/services/b.py"),
            ],
            duplication: vec![
                clone_class(&["/repo/packages/a.py", "/repo/shared/copy.py"]),
                clone_class(&["/repo/services/b.py", "/repo/shared/other.py"]),
            ],
            meta: ReportMeta {
                mode: ReportMode::Full,
                dead_code_total: 2,
                smells_total: 2,
                boundaries_total: 2,
                duplication_total: 2,
                ..Default::default()
            },
            ..Default::default()
        };

        apply(&mut report, root, &["packages/**".to_string()]).unwrap();

        assert_eq!(report.dead_code.len(), 1);
        assert_eq!(report.smells.len(), 1);
        assert_eq!(report.boundaries.len(), 1);
        assert_eq!(report.duplication.len(), 1);
        assert_eq!(report.meta.dead_code_total, 1);
        assert_eq!(report.meta.smells_total, 1);
        assert_eq!(report.meta.boundaries_total, 1);
        assert_eq!(report.meta.duplication_total, 1);
        assert_eq!(report.meta.mode, ReportMode::Full);
    }

    #[test]
    fn negated_glob_drops_only_the_excluded_clone_occurrence() {
        let root = Path::new("/repo");
        let mut report = AnalysisReport {
            duplication: vec![clone_class(&[
                "/repo/core/util.ts",
                "/repo/forkv2/util.ts",
                "/repo/other/util.ts",
            ])],
            dead_code: vec![dead("/repo/forkv2/a.ts"), dead("/repo/core/b.ts")],
            ..Default::default()
        };

        apply(&mut report, root, &["!**/forkv2/**".to_string()]).unwrap();

        let files: Vec<_> = report.duplication[0]
            .occurrences
            .iter()
            .map(|occurrence| occurrence.file.clone())
            .collect();
        assert_eq!(
            files,
            [
                PathBuf::from("/repo/core/util.ts"),
                PathBuf::from("/repo/other/util.ts")
            ]
        );
        assert_eq!(report.dead_code.len(), 1);
        assert_eq!(report.meta.dead_code_total, 1);
    }

    #[test]
    fn clone_left_with_one_occurrence_is_dropped() {
        let mut report = AnalysisReport {
            duplication: vec![clone_class(&["/repo/core/a.ts", "/repo/forkv2/a.ts"])],
            ..Default::default()
        };

        apply(&mut report, Path::new("/repo"), &["!forkv2".to_string()]).unwrap();

        assert!(report.duplication.is_empty());
    }

    #[test]
    fn negation_that_can_never_match_is_rejected() {
        for pattern in ["!", "!!src/**"] {
            let mut report = AnalysisReport::default();
            let err = apply(&mut report, Path::new("/repo"), &[pattern.to_string()]).unwrap_err();
            assert!(err.to_string().contains("can never match"), "{err}");
        }
    }

    #[test]
    fn bare_directory_name_matches_path_component() {
        let filter = OutputPathFilter::new(Path::new("/repo"), &["packages".to_string()]).unwrap();
        assert!(filter.matches(Path::new("/repo/packages/a.py")));
        assert!(!filter.matches(Path::new("/repo/services/packages_api.py")));
    }

    fn dead(file: &str) -> DeadCodeFinding {
        DeadCodeFinding {
            action: ActionLevel::Advisory,
            module: "m".into(),
            symbol: "f".into(),
            kind: SymbolKind::Function,
            confidence: Confidence::High,
            file: file.into(),
            line: 1,
            reason: String::new(),
        }
    }

    fn smell(file: &str) -> SmellFinding {
        SmellFinding {
            action: ActionLevel::Warning,
            kind: SmellKind::LongFunction,
            message: String::new(),
            file: file.into(),
            line: 1,
            end_line: 1,
            lines: Vec::new(),
            symbol: "f".into(),
            severity: Severity::Warning,
            metric: 0,
            threshold: 0,
            reason: String::new(),
        }
    }

    fn boundary(file: &str) -> BoundaryViolation {
        BoundaryViolation {
            action: ActionLevel::MustFix,
            from_module: "a".into(),
            to_module: "b".into(),
            file: file.into(),
            line: 1,
            rule: "a -x-> b".into(),
        }
    }

    fn clone_class(files: &[&str]) -> CloneClass {
        CloneClass {
            action: ActionLevel::Advisory,
            token_length: 50,
            occurrences: files
                .iter()
                .map(|file| CloneOccurrence {
                    file: (*file).into(),
                    start_row: 1,
                    end_row: 2,
                })
                .collect(),
            hint: None,
            ..Default::default()
        }
    }
}

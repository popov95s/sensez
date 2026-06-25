//! Presentation-only report filtering. This never changes scan scope: analyzers
//! run over the full project first, then the rendered/serialized report is
//! narrowed to findings whose source paths match the requested globs.

use crate::report::AnalysisReport;
use anyhow::Result;
use globset::{Glob, GlobSet};
use std::path::{Path, PathBuf};

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
    report.duplication.retain(|class| {
        class
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

    report.meta.cycles_total = report.cycles.len();
    report.meta.dead_code_total = report.dead_code.len();
    report.meta.boundaries_total = report.boundaries.len();
    report.meta.duplication_total = report.duplication.len();
    report.meta.smells_total = report.smells.len();
    report.meta.smell_totals = smell_totals(&report.smells);
    report.meta.files_skipped = report.meta.issues.len();
    report.meta.glossary = crate::noze::glossary::for_report(report);
    Ok(())
}

fn smell_totals(
    smells: &[crate::report::SmellFinding],
) -> std::collections::BTreeMap<String, usize> {
    let mut totals = std::collections::BTreeMap::new();
    for smell in smells {
        *totals.entry(smell.kind.as_str().to_string()).or_default() += 1;
    }
    totals
}

struct OutputPathFilter {
    root: PathBuf,
    globs: GlobSet,
    component_patterns: Vec<String>,
}

impl OutputPathFilter {
    fn new(root: &Path, patterns: &[String]) -> Result<Self> {
        let mut builder = GlobSet::builder();
        let mut component_patterns = Vec::new();
        for pattern in patterns {
            builder.add(Glob::new(pattern)?);
            if is_single_literal_component(pattern) {
                component_patterns.push(pattern.to_string());
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            globs: builder.build()?,
            component_patterns,
        })
    }

    fn matches(&self, path: &Path) -> bool {
        self.glob_matches(path) || self.component_matches(path)
    }

    fn glob_matches(&self, path: &Path) -> bool {
        if self.globs.is_match(path) {
            return true;
        }
        path.strip_prefix(&self.root)
            .ok()
            .is_some_and(|relative| self.globs.is_match(relative))
    }

    fn component_matches(&self, path: &Path) -> bool {
        self.component_patterns
            .iter()
            .any(|pattern| path_has_component(path, pattern) || relative_has_component(path, &self.root, pattern))
    }
}

fn path_has_component(path: &Path, pattern: &str) -> bool {
    path.components()
        .any(|c| c.as_os_str() == pattern)
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
        DeadCodeFinding, ReportMode, Severity, SmellFinding, SmellKind,
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
            meta: crate::report::ReportMeta {
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
        }
    }
}

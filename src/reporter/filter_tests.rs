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
fn single_star_stays_within_one_directory() {
    let mut report = AnalysisReport {
        dead_code: vec![dead("/repo/pkg/a.py"), dead("/repo/pkg/sub/b.py")],
        ..Default::default()
    };

    apply(&mut report, Path::new("/repo"), &["pkg/*.py".to_string()]).unwrap();

    assert_eq!(report.dead_code.len(), 1, "{:?}", report.dead_code);
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

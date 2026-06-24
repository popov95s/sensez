use super::*;
use crate::report::{
    ActionLevel, AnalysisReport, BoundaryViolation, CloneClass, CloneOccurrence, Confidence,
    CycleEdge, CycleFinding, DeadCodeFinding, ReportMeta, ReportMode, Severity, SmellFinding,
    SmellKind,
};
use crate::spine::ir::SymbolKind;
use clap::Parser;
use std::path::PathBuf;

#[test]
fn fail_on_new_accepts_project_level_spelling() {
    for value in ["must_fix", "must-fix"] {
        let cli = match spec::Cli::try_parse_from(["sensez", "noze", ".", "--fail-on-new", value]) {
            Ok(cli) => cli,
            Err(err) => panic!("failed to parse --fail-on-new {value}: {err}"),
        };
        let Some(Command::Noze(args)) = cli.command else {
            panic!("expected noze command");
        };
        assert_eq!(args.options.fail_on_new, Some(FailOnNewLevel::MustFix));
    }
}

#[test]
fn fail_on_new_without_value_defaults_to_must_fix() {
    let cli = match spec::Cli::try_parse_from(["sensez", "noze", ".", "--fail-on-new"]) {
        Ok(cli) => cli,
        Err(err) => panic!("failed to parse bare --fail-on-new: {err}"),
    };
    let Some(Command::Noze(args)) = cli.command else {
        panic!("expected noze command");
    };
    assert_eq!(args.options.fail_on_new, Some(FailOnNewLevel::MustFix));
}

#[test]
fn bare_path_defaults_to_noze_scan() {
    let cli = spec::Cli::try_parse_from(["sensez", "."]).unwrap();
    assert!(cli.command.is_none());
    assert_eq!(cli.path, Some(PathBuf::from(".")));
}

#[test]
fn pillar_flags_can_be_combined() {
    let cli = spec::Cli::try_parse_from(["sensez", "noze", "--duplicates", "--dead-code"]).unwrap();
    let Some(Command::Noze(args)) = cli.command else {
        panic!("expected noze command");
    };
    assert!(args.options.duplicates);
    assert!(args.options.dead_code);
    assert!(!args.options.cycles);
}

#[test]
fn default_output_keeps_high_confidence_dead_code_only() {
    let mut report = AnalysisReport::default();
    report.dead_code.push(dead("sure", Confidence::High));
    report.dead_code.push(dead("maybe", Confidence::Medium));
    let options = spec::ScanOptions {
        threshold: None,
        summary: false,
        json: false,
        max: None,
        all: false,
        duplicates: false,
        dead_code: false,
        cycles: false,
        boundaries: false,
        smells: false,
        output_glob: Vec::new(),
        diff: false,
        diff_from: None,
        fail_on_new: None,
        explain: false,
    };

    output::apply(&mut report, &options);

    assert_eq!(report.dead_code.len(), 1);
    assert_eq!(report.dead_code[0].symbol, "sure");
    assert_eq!(report.meta.dead_code_total, 1);
}

#[test]
fn pillar_filter_keeps_only_requested_findings() {
    let mut report = AnalysisReport::default();
    report.dead_code.push(dead("sure", Confidence::High));
    report.duplication.push(CloneClass {
        action: ActionLevel::Advisory,
        token_length: 1,
        occurrences: vec![CloneOccurrence {
            file: PathBuf::from("a.py"),
            start_row: 1,
            end_row: 1,
        }],
        hint: None,
    });
    let options = spec::ScanOptions {
        threshold: None,
        summary: false,
        json: false,
        max: None,
        all: false,
        duplicates: true,
        dead_code: false,
        cycles: false,
        boundaries: false,
        smells: false,
        output_glob: Vec::new(),
        diff: false,
        diff_from: None,
        fail_on_new: None,
        explain: false,
    };

    output::apply(&mut report, &options);

    assert!(report.dead_code.is_empty());
    assert_eq!(report.duplication.len(), 1);
}

#[test]
fn fail_on_new_blocks_at_configured_level() {
    let mut report = AnalysisReport {
        meta: ReportMeta {
            mode: ReportMode::Diff,
            ..ReportMeta::default()
        },
        ..AnalysisReport::default()
    };
    report.smells.push(SmellFinding {
        action: ActionLevel::Advisory,
        kind: SmellKind::LongFunction,
        message: String::new(),
        file: PathBuf::from("a.py"),
        line: 1,
        end_line: 1,
        symbol: "f".into(),
        severity: Severity::Warning,
        metric: 1,
        threshold: 1,
        reason: String::new(),
    });
    assert!(!report_meets_fail_level(&report, FailOnNewLevel::MustFix));
    assert!(!report_meets_fail_level(&report, FailOnNewLevel::Warning));
    report.smells[0].action = ActionLevel::Warning;
    assert!(report_meets_fail_level(&report, FailOnNewLevel::Warning));
    report.smells[0].action = ActionLevel::MustFix;
    assert!(report_meets_fail_level(&report, FailOnNewLevel::MustFix));

    let _ = (
        CycleFinding {
            action: ActionLevel::Advisory,
            modules: vec![],
            edges: vec![CycleEdge {
                from_module: String::new(),
                to_module: String::new(),
                file: PathBuf::from("a.py"),
                line: 1,
            }],
        },
        DeadCodeFinding {
            action: ActionLevel::Advisory,
            module: String::new(),
            symbol: String::new(),
            kind: SymbolKind::Function,
            confidence: Confidence::High,
            file: PathBuf::from("a.py"),
            line: 1,
            reason: String::new(),
        },
        BoundaryViolation {
            action: ActionLevel::Advisory,
            from_module: String::new(),
            to_module: String::new(),
            file: PathBuf::from("a.py"),
            line: 1,
            rule: String::new(),
        },
        CloneClass {
            action: ActionLevel::Advisory,
            token_length: 1,
            occurrences: vec![CloneOccurrence {
                file: PathBuf::from("a.py"),
                start_row: 1,
                end_row: 1,
            }],
            hint: None,
        },
    );
}

fn dead(symbol: &str, confidence: Confidence) -> DeadCodeFinding {
    DeadCodeFinding {
        action: ActionLevel::Advisory,
        module: "m".into(),
        symbol: symbol.into(),
        kind: SymbolKind::Function,
        confidence,
        file: PathBuf::from("a.py"),
        line: 1,
        reason: String::new(),
    }
}

#[test]
fn diff_selection_degrades_to_warning_outside_git() {
    let tmp = tempfile::tempdir().unwrap();
    let selected = build_diff(tmp.path(), true, None);
    assert!(selected.changed.is_none());
    assert_eq!(selected.issues.len(), 1);
    assert_eq!(selected.issues[0].stage, crate::report::ScanStage::Diff);
    assert!(
        selected.issues[0].message.contains("git rev-parse"),
        "{:?}",
        selected.issues[0]
    );
}

use sensez::config::{
    model::{Config, Duplication},
    smells::{SmellConfig, Smells},
};
use sensez::{
    analyze_path, scan, ActionLevel, AnalysisReport, Confidence, Format, GlossaryEntry, ReportMeta,
    ReportMode, ScanIssue, ScanStage, Severity, SmellKind,
};
use std::fs;

#[test]
fn public_report_and_config_surface_is_constructible() {
    let _config = Config::default();
    let _duplication = Duplication::default();
    let _smell_config = SmellConfig::default();
    let _smells = Smells::default();

    let report = AnalysisReport {
        meta: ReportMeta {
            mode: ReportMode::Diff,
            issues: vec![ScanIssue {
                stage: ScanStage::Parse,
                file: None,
                message: "bad file".to_string(),
            }],
            glossary: vec![GlossaryEntry {
                term: "god_module".to_string(),
                title: "God Module".to_string(),
                explanation: "A dependency hotspot.".to_string(),
            }],
            ..ReportMeta::default()
        },
        ..AnalysisReport::default()
    };

    let json = serde_json::to_value(report).unwrap();
    assert_eq!(json["meta"]["mode"], "diff");
    assert_eq!(SmellKind::GodModule.as_str(), "god_module");
    assert_eq!(ActionLevel::MustFix.as_str(), "must_fix");
    assert!(matches!(Confidence::High, Confidence::High));
    assert!(matches!(Severity::Warning, Severity::Warning));
}

#[test]
fn public_scan_entry_points_work_on_a_tiny_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("demo.py"), "def add(a, b):\n    return a + b\n").unwrap();

    let rendered = scan(root, None, Format::Json, 0).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(parsed["meta"]["mode"], "full");

    let (report, _) = analyze_path(root, None).unwrap();
    assert_eq!(report.meta.mode, ReportMode::Full);
    assert_eq!(report.meta.analyzed_files, 1);
}

#[test]
fn scan_degrades_to_defaults_when_config_is_invalid() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("sensez.toml"), "exclude = [\"[invalid\"]\n").unwrap();
    fs::write(root.join("demo.py"), "def add(a, b):\n    return a + b\n").unwrap();

    let (report, _) = analyze_path(root, None).unwrap();
    assert_eq!(report.meta.analyzed_files, 1);
    assert!(report.meta.issues.iter().any(|issue| {
        issue.stage == ScanStage::Config && issue.message.contains("invalid glob in exclude")
    }));
}

#[test]
fn scan_warns_when_pyproject_config_cannot_be_read() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("pyproject.toml"), "[tool.sensez\n").unwrap();
    fs::write(root.join("demo.py"), "def add(a, b):\n    return a + b\n").unwrap();

    let (report, _) = analyze_path(root, None).unwrap();
    assert_eq!(report.meta.analyzed_files, 1);
    assert!(report.meta.issues.iter().any(|issue| {
        issue.stage == ScanStage::Config && issue.message.contains("parsing pyproject.toml")
    }));
}

use sensez::{
    analyze_path, ActionLevel, AnalysisReport, BoundaryViolation, CloneClass, Confidence,
    CycleFinding, DeadCodeFinding, Severity, SmellFinding, SmellKind,
};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    root: Option<PathBuf>,
    #[serde(default)]
    meta: MetaExpect,
    #[serde(default)]
    cycles: ExpectSet<CycleExpect>,
    #[serde(default)]
    boundaries: ExpectSet<BoundaryExpect>,
    #[serde(default)]
    dead_code: ExpectSet<DeadCodeExpect>,
    #[serde(default)]
    duplication: ExpectSet<DuplicationExpect>,
    #[serde(default)]
    smells: ExpectSet<SmellExpect>,
}

#[derive(Debug, Default, Deserialize)]
struct MetaExpect {
    issues_empty: Option<bool>,
    boundaries_configured: Option<bool>,
    min_internal_edges: Option<usize>,
    min_analyzed_files: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct ExpectSet<T> {
    #[serde(default)]
    contains: Vec<T>,
    #[serde(default)]
    absent: Vec<T>,
}

impl<T> Default for ExpectSet<T> {
    fn default() -> Self {
        Self {
            contains: Vec::new(),
            absent: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct CycleExpect {
    modules: Vec<String>,
    action: Option<ActionLevel>,
}

#[derive(Debug, Deserialize)]
struct BoundaryExpect {
    from: String,
    to: String,
    rule: Option<String>,
    action: Option<ActionLevel>,
}

#[derive(Debug, Deserialize)]
struct DeadCodeExpect {
    module: Option<String>,
    symbol: String,
    confidence: Option<String>,
    action: Option<ActionLevel>,
}

#[derive(Debug, Deserialize)]
struct DuplicationExpect {
    files: Vec<String>,
    min_token_length: Option<usize>,
    action: Option<ActionLevel>,
}

#[derive(Debug, Deserialize)]
struct SmellExpect {
    kind: SmellKind,
    symbol: Option<String>,
    action: Option<ActionLevel>,
    severity: Option<String>,
}

struct LoadedCase {
    name: String,
    source: PathBuf,
    config: Case,
}

#[test]
fn fixture_cases_match_expectations() {
    let cases = load_cases();
    assert!(!cases.is_empty(), "expected at least one fixture case");

    for case in cases {
        let tmp = tempfile::tempdir().unwrap();
        let copied = tmp.path().join(
            case.source
                .file_name()
                .expect("case source has a final path component"),
        );
        copy_dir_all(&case.source, &copied);
        let scan_root = copied.join(case.config.root.as_deref().unwrap_or(Path::new(".")));
        let report = analyze_path(&scan_root, None, None)
            .unwrap_or_else(|err| panic!("{}: scan failed: {err:#}", case.name));

        assert_case(&case.config, &report);
    }
}

fn load_cases() -> Vec<LoadedCase> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample");
    let mut dirs: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("case.toml").exists())
        .collect();
    dirs.sort();

    dirs.into_iter()
        .map(|source| {
            let text = fs::read_to_string(source.join("case.toml")).unwrap();
            let config: Case = toml::from_str(&text)
                .unwrap_or_else(|err| panic!("{}: invalid case.toml: {err}", source.display()));
            let name = config.name.clone();
            LoadedCase {
                name,
                source,
                config,
            }
        })
        .collect()
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let ty = entry.file_type().unwrap();
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn assert_case(case: &Case, report: &AnalysisReport) {
    assert_meta(case, report);
    assert_cycles(case, report);
    assert_boundaries(case, report);
    assert_dead_code(case, report);
    assert_duplication(case, report);
    assert_smells(case, report);
}

fn assert_meta(case: &Case, report: &AnalysisReport) {
    if case.meta.issues_empty == Some(true) {
        assert!(
            report.meta.issues.is_empty(),
            "{}: expected no scan issues, got {:?}",
            case.name,
            report.meta.issues
        );
    }
    if let Some(expected) = case.meta.boundaries_configured {
        assert_eq!(
            report.meta.boundaries_configured, expected,
            "{}: boundaries_configured mismatch",
            case.name
        );
    }
    if let Some(min) = case.meta.min_internal_edges {
        assert!(
            report.meta.internal_edges >= min,
            "{}: expected at least {min} internal edge(s), got {}",
            case.name,
            report.meta.internal_edges
        );
    }
    if let Some(min) = case.meta.min_analyzed_files {
        assert!(
            report.meta.analyzed_files >= min,
            "{}: expected at least {min} analyzed file(s), got {}",
            case.name,
            report.meta.analyzed_files
        );
    }
}

fn assert_cycles(case: &Case, report: &AnalysisReport) {
    for expected in &case.cycles.contains {
        let found = report
            .cycles
            .iter()
            .find(|cycle| cycle_matches(cycle, expected))
            .unwrap_or_else(|| {
                panic!(
                    "{}: missing cycle containing {:?}; got {:?}",
                    case.name, expected.modules, report.cycles
                )
            });
        assert_optional_action(case, "cycle", expected.action, found.action);
    }
    for expected in &case.cycles.absent {
        assert!(
            !report
                .cycles
                .iter()
                .any(|cycle| cycle_matches(cycle, expected)),
            "{}: unexpected cycle containing {:?}",
            case.name,
            expected.modules
        );
    }
}

fn cycle_matches(cycle: &CycleFinding, expected: &CycleExpect) -> bool {
    expected
        .modules
        .iter()
        .all(|module| cycle.modules.iter().any(|m| m == module))
}

fn assert_boundaries(case: &Case, report: &AnalysisReport) {
    for expected in &case.boundaries.contains {
        let found = report
            .boundaries
            .iter()
            .find(|boundary| boundary_matches(boundary, expected))
            .unwrap_or_else(|| {
                panic!(
                    "{}: missing boundary {} -> {}; got {:?}",
                    case.name, expected.from, expected.to, report.boundaries
                )
            });
        assert_optional_action(case, "boundary", expected.action, found.action);
    }
    for expected in &case.boundaries.absent {
        assert!(
            !report
                .boundaries
                .iter()
                .any(|boundary| boundary_matches(boundary, expected)),
            "{}: unexpected boundary {} -> {}",
            case.name,
            expected.from,
            expected.to
        );
    }
}

fn boundary_matches(boundary: &BoundaryViolation, expected: &BoundaryExpect) -> bool {
    boundary.from_module == expected.from
        && boundary.to_module == expected.to
        && expected
            .rule
            .as_ref()
            .is_none_or(|rule| boundary.rule == *rule)
}

fn assert_dead_code(case: &Case, report: &AnalysisReport) {
    for expected in &case.dead_code.contains {
        let found = report
            .dead_code
            .iter()
            .find(|finding| dead_code_matches(finding, expected))
            .unwrap_or_else(|| {
                panic!(
                    "{}: missing dead-code symbol {}; got {:?}",
                    case.name, expected.symbol, report.dead_code
                )
            });
        assert_optional_action(case, "dead_code", expected.action, found.action);
        if let Some(confidence) = &expected.confidence {
            assert_eq!(
                confidence_name(found.confidence),
                confidence,
                "{}: dead-code confidence mismatch for {}",
                case.name,
                expected.symbol
            );
        }
    }
    for expected in &case.dead_code.absent {
        assert!(
            !report
                .dead_code
                .iter()
                .any(|finding| dead_code_matches(finding, expected)),
            "{}: unexpected dead-code symbol {}",
            case.name,
            expected.symbol
        );
    }
}

fn dead_code_matches(finding: &DeadCodeFinding, expected: &DeadCodeExpect) -> bool {
    finding.symbol == expected.symbol
        && expected
            .module
            .as_ref()
            .is_none_or(|module| finding.module == *module)
}

fn assert_duplication(case: &Case, report: &AnalysisReport) {
    for expected in &case.duplication.contains {
        let found = report
            .duplication
            .iter()
            .find(|class| duplication_matches(class, expected))
            .unwrap_or_else(|| {
                panic!(
                    "{}: missing duplication touching {:?}; got {:?}",
                    case.name, expected.files, report.duplication
                )
            });
        assert_optional_action(case, "duplication", expected.action, found.action);
        if let Some(min) = expected.min_token_length {
            assert!(
                found.token_length >= min,
                "{}: expected duplicate token length >= {min}, got {}",
                case.name,
                found.token_length
            );
        }
    }
    for expected in &case.duplication.absent {
        assert!(
            !report
                .duplication
                .iter()
                .any(|class| duplication_matches(class, expected)),
            "{}: unexpected duplication touching {:?}",
            case.name,
            expected.files
        );
    }
}

fn duplication_matches(class: &CloneClass, expected: &DuplicationExpect) -> bool {
    expected.files.iter().all(|suffix| {
        class
            .occurrences
            .iter()
            .any(|occurrence| path_ends_with(&occurrence.file, suffix))
    })
}

fn assert_smells(case: &Case, report: &AnalysisReport) {
    for expected in &case.smells.contains {
        let found = report
            .smells
            .iter()
            .find(|smell| smell_matches(smell, expected))
            .unwrap_or_else(|| {
                panic!(
                    "{}: missing smell {:?} on {:?}; got {:?}",
                    case.name, expected.kind, expected.symbol, report.smells
                )
            });
        assert_optional_action(case, "smell", expected.action, found.action);
        if let Some(severity) = &expected.severity {
            assert_eq!(
                severity_name(found.severity),
                severity,
                "{}: smell severity mismatch for {:?} on {:?}",
                case.name,
                expected.kind,
                expected.symbol
            );
        }
    }
    for expected in &case.smells.absent {
        assert!(
            !report
                .smells
                .iter()
                .any(|smell| smell_matches(smell, expected)),
            "{}: unexpected smell {:?} on {:?}",
            case.name,
            expected.kind,
            expected.symbol
        );
    }
}

fn smell_matches(smell: &SmellFinding, expected: &SmellExpect) -> bool {
    smell.kind == expected.kind
        && expected
            .symbol
            .as_ref()
            .is_none_or(|symbol| smell.symbol == *symbol)
}

fn assert_optional_action(
    case: &Case,
    label: &str,
    expected: Option<ActionLevel>,
    actual: ActionLevel,
) {
    if let Some(expected) = expected {
        assert_eq!(actual, expected, "{}: {label} action mismatch", case.name);
    }
}

fn path_ends_with(path: &Path, suffix: &str) -> bool {
    path.to_string_lossy().replace('\\', "/").ends_with(suffix)
}

fn confidence_name(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "High",
        Confidence::Medium => "Medium",
        Confidence::Low => "Low",
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "Critical",
        Severity::Warning => "Warning",
        Severity::Info => "Info",
    }
}

use super::*;
use crate::brainz::{record_scan, triage_finding, Origin};
use serde_json::Value;
use std::fs;
use std::process::Command;

#[test]
fn triaged_finding_is_suppressed_others_kept() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    if !init_repo(root) {
        return;
    }
    fs::write(
        root.join("m.py"),
        "def live():\n    return 0\n\n\ndef orphan():\n    return 1\n\n\ndef orphan_two():\n    return 2\n",
    )
    .unwrap();
    fs::write(root.join("consumer.py"), "from m import live\n\nlive()\n").unwrap();

    let text = crate::scan(root, None, crate::reporter::Format::Json, 0).unwrap();
    let baseline: Value = serde_json::from_str(&text).unwrap();
    record_scan(
        root,
        crate::diff::git::current_branch(root).as_deref(),
        &baseline,
        std::time::Duration::from_millis(1),
        None,
        Origin::Tool,
    );
    triage_finding(
        root,
        crate::report::Pillar::DeadCode,
        "orphan_two",
        crate::brainz::TriageVerdict::FalsePositive,
        None,
    )
    .unwrap();

    let (mut report, _) = crate::analyze_path(root, None).unwrap();
    apply_suppressions(root, &BTreeMap::new(), &mut report);
    let symbols: Vec<&str> = report.dead_code.iter().map(|f| f.symbol.as_str()).collect();
    assert!(
        symbols.contains(&"orphan"),
        "untriaged finding kept: {symbols:?}"
    );
    assert!(
        !symbols.contains(&"orphan_two"),
        "triaged finding suppressed: {symbols:?}"
    );
}

fn init_repo(root: &Path) -> bool {
    let initialized = Command::new("git")
        .arg("init")
        .current_dir(root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if !initialized {
        return false;
    }
    fs::write(root.join("base.py"), "print('base')\n").unwrap();
    let added = Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    added
        && Command::new("git")
            .args([
                "-c",
                "user.email=sensez@example.test",
                "-c",
                "user.name=Sensez",
                "commit",
                "-m",
                "base",
            ])
            .current_dir(root)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
}

#[test]
fn committed_accept_list_suppresses_by_label() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("m.py"),
        "def live():\n    return 0\n\n\ndef orphan():\n    return 1\n\n\ndef keeper():\n    return 2\n",
    )
    .unwrap();
    fs::write(root.join("consumer.py"), "from m import live\n\nlive()\n").unwrap();
    fs::write(
        root.join("sensez.toml"),
        "[accept]\ndead_code = [\"m::orphan\"]\n",
    )
    .unwrap();

    let (report, _) = crate::analyze_path(root, None).unwrap();
    let symbols: Vec<&str> = report.dead_code.iter().map(|f| f.symbol.as_str()).collect();
    assert!(
        symbols.contains(&"keeper"),
        "unlisted finding kept: {symbols:?}"
    );
    assert!(
        !symbols.contains(&"orphan"),
        "accept-listed finding suppressed: {symbols:?}"
    );
}

#[test]
fn accepted_duplication_path_drops_only_that_occurrence() {
    let tmp = tempfile::tempdir().unwrap();
    let mut report = three_way_clone();

    apply_suppressions(tmp.path(), &accept_duplication("forkv2/"), &mut report);

    let files: Vec<_> = report.duplication[0]
        .occurrences
        .iter()
        .map(|occurrence| occurrence.file.to_string_lossy().into_owned())
        .collect();
    assert_eq!(files, ["core/util.ts", "other/util.ts"]);
}

#[test]
fn accepted_duplication_path_matches_whole_components() {
    let tmp = tempfile::tempdir().unwrap();
    let mut report = three_way_clone();

    apply_suppressions(tmp.path(), &accept_duplication("v2/"), &mut report);

    assert_eq!(report.duplication[0].occurrences.len(), 3);
}

#[test]
fn accepted_duplication_label_hides_the_whole_group() {
    let tmp = tempfile::tempdir().unwrap();
    let mut report = three_way_clone();
    let label = "core/util.ts + forkv2/util.ts + other/util.ts";

    apply_suppressions(tmp.path(), &accept_duplication(label), &mut report);

    assert!(report.duplication.is_empty());
}

fn accept_duplication(pattern: &str) -> BTreeMap<String, Vec<String>> {
    BTreeMap::from([("duplication".to_string(), vec![pattern.to_string()])])
}

fn three_way_clone() -> crate::report::AnalysisReport {
    let occurrence = |file: &str| crate::report::CloneOccurrence {
        file: file.into(),
        start_row: 1,
        end_row: 20,
    };
    crate::report::AnalysisReport {
        duplication: vec![CloneClass {
            token_length: 77,
            occurrences: vec![
                occurrence("core/util.ts"),
                occurrence("forkv2/util.ts"),
                occurrence("other/util.ts"),
            ],
            ..CloneClass::default()
        }],
        ..crate::report::AnalysisReport::default()
    }
}

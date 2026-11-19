use crate::brainz::fingerprint::{Aged, ResolvedHistory};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn slow_repos_wait_for_the_rescan_interval_and_fast_ones_do_not() {
    let base = |ms| super::hub::Baseline {
        ts: 1_000,
        ms,
        threshold: None,
        branch: "main".to_string(),
        source_state: None,
    };
    let soon = 1_000 + super::SLOW_RESCAN_INTERVAL_SECS - 1;
    let later = 1_000 + super::SLOW_RESCAN_INTERVAL_SECS;

    assert!(super::rescan_due(&base(super::SLOW_SCAN_MS - 1), soon));
    assert!(!super::rescan_due(&base(super::SLOW_SCAN_MS), soon));
    assert!(super::rescan_due(&base(super::SLOW_SCAN_MS), later));
}

/// Scan → fix → auto-recapture (no second scan call, no agent report).
#[test]
fn recapture_banks_resolved_without_a_second_scan() {
    let _metrics = crate::test_support::metrics_guard();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    fs::create_dir_all(&root).unwrap();
    if init_repo(&root).is_none() {
        return;
    }
    fs::write(
        root.join("app.py"),
        "def used():\n    return 1\n\ndef orphan():\n    return 2\n\nprint(used())\n",
    )
    .unwrap();
    fs::write(root.join("consumer.py"), "from app import used\n\nused()\n").unwrap();

    let text = crate::scan(&root, None, crate::reporter::Format::Json, 0).unwrap();
    let report: Value = serde_json::from_str(&text).unwrap();
    crate::brainz::record_scan(
        &root,
        crate::diff::git::current_branch(&root).as_deref(),
        &report,
        std::time::Duration::from_millis(1),
        None,
        crate::brainz::Origin::Tool,
    );

    fs::write(
        root.join("app.py"),
        "def used():\n    return 1\n\nprint(used())\n",
    )
    .unwrap();
    super::run();

    let totals = crate::brainz::usage_report(&root);
    assert_eq!(
        totals["all_time"]["resolved_by_detector"]["dead_code/function"]["count"], 1,
        "auto-recapture must record the deleted orphan as resolved"
    );
}

#[test]
fn stale_recapture_baseline_is_a_noop() {
    let _metrics = crate::test_support::metrics_guard();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    fs::create_dir_all(&root).unwrap();
    let Some(branch) = init_repo(&root) else {
        return;
    };
    fs::write(
        root.join("app.py"),
        "def used():\n    return 0\n\n\ndef orphan():\n    return 1\n",
    )
    .unwrap();
    fs::write(root.join("consumer.py"), "from app import used\n\nused()\n").unwrap();

    let text = crate::scan(&root, None, crate::reporter::Format::Json, 0).unwrap();
    let report: Value = serde_json::from_str(&text).unwrap();
    crate::brainz::record_scan(
        &root,
        crate::diff::git::current_branch(&root).as_deref(),
        &report,
        std::time::Duration::from_millis(1),
        None,
        crate::brainz::Origin::Tool,
    );
    crate::brainz::flush();

    super::store::save_fingerprints(
        &root,
        &branch,
        &Aged::new(),
        &ResolvedHistory::new(),
        u64::MAX,
    )
    .unwrap();

    super::recapture(&root, &branch, None, 1);

    let report = crate::brainz::usage_report(&root);
    assert!(
        report["all_time"]["resolved_by_detector"]
            .as_object()
            .unwrap()
            .is_empty(),
        "a stale process must not resolve from an obsolete baseline"
    );
    assert!(
        report["all_time"]["reintroduced_by_detector"]
            .as_object()
            .unwrap()
            .is_empty(),
        "a stale process must not reintroduce from an obsolete baseline"
    );
}

fn init_repo(root: &Path) -> Option<String> {
    let initialized = Command::new("git")
        .arg("init")
        .current_dir(root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if !initialized {
        return None;
    }
    fs::write(root.join("base.py"), "print('base')\n").unwrap();
    let added = Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if !added {
        return None;
    }
    let committed = Command::new("git")
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
        .unwrap_or(false);
    if !committed {
        return None;
    }
    let out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!branch.is_empty() && branch != "HEAD").then_some(branch)
}

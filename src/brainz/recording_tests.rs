use super::*;
use crate::test_support::GitTestRepo;
use serde_json::json;

#[test]
fn unnamed_branch_scan_does_not_diff_against_shared_baseline() {
    let _metrics = crate::test_support::metrics_guard();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let first = json!({
        "dead_code": [{"module": "demo", "symbol": "alpha", "kind": "function"}],
    });
    let second = json!({"dead_code": []});

    record_scan(
        root,
        crate::diff::git::current_branch(root).as_deref(),
        &first,
        Duration::from_millis(1),
        None,
        Origin::Tool,
    );
    record_scan(
        root,
        crate::diff::git::current_branch(root).as_deref(),
        &second,
        Duration::from_millis(1),
        None,
        Origin::Tool,
    );
    crate::brainz::flush();

    let totals = store::load_totals(root);
    assert!(totals.resolved_by_detector.is_empty());
    assert!(totals.reintroduced_by_detector.is_empty());
    assert!(!store::dir(root).join("baselines.json").exists());
}

#[test]
fn detached_scan_does_not_reuse_a_named_branch_baseline() {
    let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = GitTestRepo::new("unused.py", "print('base')\n") else {
        return;
    };
    let root = repo.root.as_path();
    let seen = json!({
        "dead_code": [{"module": "demo", "symbol": "kept", "kind": "function"}],
    });
    let gone = json!({"dead_code": []});

    record_scan(
        root,
        crate::diff::git::current_branch(root).as_deref(),
        &seen,
        Duration::from_millis(1),
        None,
        Origin::Tool,
    );
    crate::brainz::flush();
    assert!(store::dir(root).join("baselines.json").exists());

    assert!(repo.git(&["checkout", "--detach"]));
    record_scan(
        root,
        crate::diff::git::current_branch(root).as_deref(),
        &gone,
        Duration::from_millis(1),
        None,
        Origin::Tool,
    );
    crate::brainz::flush();

    let totals = store::load_totals(root);
    assert!(totals.resolved_by_detector.is_empty());
    assert!(totals.reintroduced_by_detector.is_empty());
    assert!(store::load_fingerprints(root, "").is_empty());
    assert!(store::load_resolved_history(root, "").is_empty());
}

#[test]
fn scan_is_recorded_under_the_branch_the_caller_resolved() {
    let _metrics = crate::test_support::metrics_guard();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let found = json!({
        "dead_code": [{"module": "demo", "symbol": "alpha", "kind": "function"}],
    });

    record_scan(
        root,
        Some("feature/x"),
        &found,
        Duration::from_millis(1),
        None,
        Origin::Tool,
    );
    crate::brainz::flush();

    let prints = store::load_fingerprints(root, "feature/x");
    assert_eq!(prints.values().map(|pillar| pillar.len()).sum::<usize>(), 1);
    assert!(hub::baselines()
        .iter()
        .any(|(path, baseline)| path == root && baseline.branch == "feature/x"));
}

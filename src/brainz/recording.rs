//! Event recording entry points for scans, searches, gates, and human verdicts.

use super::events::{Event, Origin};
use super::fingerprint::{Detector, Pillar};
use super::hub::{self, Baseline};
use super::verdict::TriageVerdict;
use super::{aging, fingerprint, store, triage};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

pub fn record_scan(
    root: &Path,
    branch: Option<&str>,
    report: &Value,
    elapsed: Duration,
    threshold: Option<usize>,
    origin: Origin,
) {
    if !hub::enabled(root) {
        return;
    }
    let reported = fingerprint::detector_counts(report);
    let meta_u64 = |key: &str| {
        report
            .get("meta")
            .and_then(|m| m.get(key))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let files = meta_u64("analyzed_files");
    let loc = meta_u64("source_lines");
    let config_hash = crate::config::model::Config::load(root)
        .ok()
        .map(|c| c.signature());
    let current = fingerprint::fingerprints(report);
    let now = hub::now();
    let (resolved, reintroduced) = if let Some(branch) = branch {
        let ignore = triage::ignored_keys(&triage::load(root));
        let mut resolved = BTreeMap::new();
        let mut reintroduced = BTreeMap::new();
        if let Err(err) = store::update_fingerprints(root, branch, now, |previous, history| {
            let aging = aging::age(&previous, &current, &history, now, &ignore);
            resolved = aging.resolved;
            reintroduced = aging.reintroduced;
            (aging.aged, aging.history)
        }) {
            eprintln!("[sensez metrics] saving fingerprints: {err:#}");
        }
        hub::set_baseline(
            root,
            Baseline {
                ts: now,
                ms: elapsed.as_millis() as u64,
                threshold,
                branch: branch.to_string(),
                source_state: hub::observed_source_state(root),
            },
        );
        (resolved, reintroduced)
    } else {
        (BTreeMap::new(), BTreeMap::new())
    };
    hub::push_on(root, branch, move |session, branch| Event::Scan {
        ts: now,
        session,
        branch,
        ms: elapsed.as_millis() as u64,
        origin,
        reported,
        resolved,
        reintroduced,
        files,
        loc,
        config_hash,
    });
}

pub fn triage_finding(
    root: &Path,
    pillar: Pillar,
    pattern: &str,
    verdict: TriageVerdict,
    note: Option<String>,
) -> anyhow::Result<Vec<String>> {
    let branch = hub::branch_key(root)
        .ok_or_else(|| anyhow::anyhow!("triage requires a named git branch"))?;
    let marked = triage::mark(root, &branch, pillar, pattern, verdict, note.clone())?;
    if verdict != TriageVerdict::Clear {
        let mut by_detector: BTreeMap<&Detector, u64> = BTreeMap::new();
        for found in &marked {
            *by_detector.entry(&found.detector).or_default() += 1;
        }
        for (detector, count) in by_detector {
            record_outcome(root, detector, verdict, count, note.clone());
        }
    }
    Ok(marked.into_iter().map(|found| found.label).collect())
}

#[cfg(feature = "eyez")]
pub fn record_search(
    root: &Path,
    query_len: usize,
    hits: usize,
    top_score: f32,
    bytes_returned: u64,
    file_bytes_referenced: u64,
    ms: u64,
) {
    let first_on_repo = store::load_totals(root).searches == 0 && hub::session_searches(root) == 0;
    hub::push(root, |session, branch| Event::Search {
        ts: hub::now(),
        session,
        branch,
        ms,
        query_len: query_len as u64,
        hits: hits as u64,
        top_score,
        first_on_repo,
        bytes_returned,
        file_bytes_referenced,
    });
}

pub fn record_gate_block(root: &Path, branch: Option<&str>, scope: Option<&str>, report: &Value) {
    let fingerprints: Vec<String> = fingerprint::fingerprints(report)
        .values()
        .flatten()
        .map(|p| p.key())
        .collect();
    hub::push_on(root, branch, move |session, branch| Event::GateBlock {
        ts: hub::now(),
        session,
        branch,
        scope: scope.map(str::to_string),
        fingerprints,
    });
}

fn record_outcome(
    root: &Path,
    detector: &Detector,
    verdict: TriageVerdict,
    count: u64,
    detail: Option<String>,
) {
    hub::push(root, |session, branch| Event::Outcome {
        ts: hub::now(),
        session,
        branch,
        detector: detector.to_string(),
        verdict,
        count,
        detail,
    });
}

#[cfg(test)]
mod tests {
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
}

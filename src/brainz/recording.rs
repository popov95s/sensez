//! Event recording entry points for scans, searches, gates, and human verdicts.

use super::events::{Event, Origin};
use super::hub::{self, Baseline};
use super::{aging, fingerprint, store, triage};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

pub fn record_scan(
    root: &Path,
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
    let (resolved, reintroduced) = if let Some(branch) = hub::branch_key(root) {
        let previous = store::load_fingerprints(root, &branch);
        let history = store::load_resolved_history(root, &branch);
        let ignore = triage::ignored_keys(&triage::load(root));
        let aging = aging::age(&previous, &current, &history, now, &ignore);
        if let Err(err) = store::save_fingerprints(root, &branch, &aging.aged, &aging.history, now)
        {
            eprintln!("[sensez metrics] saving fingerprints: {err:#}");
        }
        hub::set_baseline(
            root,
            Baseline {
                ts: now,
                ms: elapsed.as_millis() as u64,
                threshold,
                branch,
            },
        );
        (aging.resolved, aging.reintroduced)
    } else {
        (BTreeMap::new(), BTreeMap::new())
    };
    hub::push(root, move |session, branch| Event::Scan {
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
    pillar: &str,
    pattern: &str,
    verdict: &str,
    note: Option<String>,
) -> anyhow::Result<Vec<String>> {
    let branch = hub::branch_key(root)
        .ok_or_else(|| anyhow::anyhow!("triage requires a named git branch"))?;
    let marked = triage::mark(root, &branch, pillar, pattern, verdict, note.clone())?;
    if verdict != "clear" {
        let mut by_detector: BTreeMap<String, u64> = BTreeMap::new();
        for (_, detector) in &marked {
            *by_detector.entry(detector.clone()).or_default() += 1;
        }
        for (detector, count) in by_detector {
            record_outcome(root, &detector, verdict, count, note.clone());
        }
    }
    Ok(marked.into_iter().map(|(label, _)| label).collect())
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

pub fn record_gate_block(root: &Path, report: &Value) {
    let fingerprints: Vec<String> = fingerprint::fingerprints(report)
        .values()
        .flatten()
        .map(|p| format!("{:x}", p.hash))
        .collect();
    hub::push(root, move |session, branch| Event::GateBlock {
        ts: hub::now(),
        session,
        branch,
        fingerprints,
    });
}

fn record_outcome(root: &Path, pillar: &str, action: &str, count: u64, detail: Option<String>) {
    hub::push(root, |session, branch| Event::Outcome {
        ts: hub::now(),
        session,
        branch,
        pillar: pillar.to_string(),
        action: action.to_string(),
        count,
        detail,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn unnamed_branch_scan_does_not_diff_against_shared_baseline() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let first = json!({
            "dead_code": [{"module": "demo", "symbol": "alpha", "kind": "function"}],
        });
        let second = json!({"dead_code": []});

        record_scan(root, &first, Duration::from_millis(1), None, Origin::Tool);
        record_scan(root, &second, Duration::from_millis(1), None, Origin::Tool);
        crate::brainz::flush();

        let totals = store::load_totals(root);
        assert!(totals.resolved_by_detector.is_empty());
        assert!(totals.reintroduced_by_detector.is_empty());
        assert!(!store::dir(root).join("last-scan.json").exists());
    }

    #[test]
    fn detached_scan_does_not_reuse_a_named_branch_baseline() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        if !init_repo(root) {
            return;
        }
        let seen = json!({
            "dead_code": [{"module": "demo", "symbol": "kept", "kind": "function"}],
        });
        let gone = json!({"dead_code": []});

        record_scan(root, &seen, Duration::from_millis(1), None, Origin::Tool);
        crate::brainz::flush();
        assert!(store::dir(root).join("last-scan.json").exists());

        assert!(git(root, &["checkout", "--detach"]));
        record_scan(root, &gone, Duration::from_millis(1), None, Origin::Tool);
        crate::brainz::flush();

        let totals = store::load_totals(root);
        assert!(totals.resolved_by_detector.is_empty());
        assert!(totals.reintroduced_by_detector.is_empty());
        assert!(store::load_fingerprints(root, "").is_empty());
        assert!(store::load_resolved_history(root, "").is_empty());
    }

    fn init_repo(root: &Path) -> bool {
        if !git(root, &["init"]) {
            return false;
        }
        std::fs::write(root.join("base.py"), "print('base')\n").unwrap();
        git(root, &["add", "."])
            && git(
                root,
                &[
                    "-c",
                    "user.email=sensez@example.test",
                    "-c",
                    "user.name=Sensez",
                    "commit",
                    "-m",
                    "base",
                ],
            )
    }

    fn git(root: &Path, args: &[&str]) -> bool {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }
}

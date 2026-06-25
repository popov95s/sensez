//! Event recording entry points for scans, searches, gates, and human verdicts.

use super::events::{Event, Origin};
use super::hub::{self, Baseline};
use super::{aging, fingerprint, store, triage};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub fn record_scan(root: &Path, report: &Value, ms: u64, threshold: Option<usize>, origin: Origin) {
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
    let branch = hub::branch_key(root);
    let current = fingerprint::fingerprints(report);
    let previous = store::load_fingerprints(root, &branch);
    let history = store::load_resolved_history(root, &branch);
    let ignore = triage::ignored_keys(&triage::load(root));
    let aging = aging::age(&previous, &current, &history, hub::now(), &ignore);
    let resolved = aging.resolved;
    let reintroduced = aging.reintroduced;
    if let Err(err) =
        store::save_fingerprints(root, &branch, &aging.aged, &aging.history, hub::now())
    {
        eprintln!("[sensez metrics] saving fingerprints: {err:#}");
    }
    hub::set_baseline(
        root,
        Baseline {
            ts: hub::now(),
            ms,
            threshold,
            branch: branch.clone(),
        },
    );
    hub::push(root, move |session, branch| Event::Scan {
        ts: hub::now(),
        session,
        branch,
        ms,
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
    let branch = hub::branch_key(root);
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

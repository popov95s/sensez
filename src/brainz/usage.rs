//! Assembly of the `brainz_report` payload from stored metrics.

use super::events::{Event, Totals};
use super::fingerprint::Aged;
use super::staleness::stale_entries;
use super::{flush, hub, report, store, triage};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

const RECENT_WINDOW_SECS: u64 = 30 * 86_400;

fn recent_window(events: &[Event]) -> (u64, Totals) {
    let cutoff = hub::now().saturating_sub(RECENT_WINDOW_SECS);
    let mut recent = Totals::default();
    for event in events {
        if event.ts() >= cutoff {
            recent.absorb(event);
        }
    }
    (cutoff, recent)
}

fn gate_block_sets(
    events: &[Event],
    baseline: Aged,
    branch: &str,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let blocked = events
        .iter()
        .filter_map(|e| match e {
            Event::GateBlock {
                branch: b,
                fingerprints,
                ..
            } if b == branch => Some(fingerprints.iter().cloned()),
            _ => None,
        })
        .flatten()
        .collect();
    let open = baseline.into_values().flat_map(|m| m.into_keys()).collect();
    (blocked, open)
}

pub fn usage_report(root: &Path) -> Value {
    if !hub::enabled(root) {
        return json!({
            "self_improvement": "disabled for this repo ([self_improvement] enabled = false)",
            "note": "this data is local-only and never transmitted in any case",
        });
    }
    flush::flush();
    let triaged = triage::load(root);
    let branch = hub::branch_key(root);
    let totals = store::load_totals(root);
    let config = crate::config::model::Config::load(root).unwrap_or_default();
    let events = store::load_events(root);
    let (recent_since, recent) = recent_window(&events);
    let (blocked, open) =
        gate_block_sets(&events, store::load_fingerprints(root, &branch), &branch);
    let has_baseline = store::has_baseline(root, &branch);
    let session = hub::session_snapshot(root);
    json!({
        "privacy": "local-only metrics from .sensez/local-metrics/ — never exported",
        "session": {
            "id": session.session_id,
            "started_unix": session.started,
            "totals": session.totals,
        },
        "precision_by_detector": report::precision_by_detector(&totals),
        "mean_resolution_days": report::mean_resolution_days(&totals.resolved_by_detector),
        "recidivism_by_detector": report::recidivism_by_detector(&totals),
        "gate_funnel": report::gate_funnel(&totals),
        "gate_conversion": report::gate_conversion(&blocked, &open, has_baseline),
        "search_health": report::search_health(&totals),
        "self_health": report::self_health(&totals),
        "config_pressure": report::config_pressure(&totals, &config),
        "calibration": report::calibration_suggestions(&totals, &config),
        "recent_30d": {
            "since_unix": recent_since,
            "scans": recent.scans,
            "precision_by_detector": report::precision_by_detector(&recent),
            "recidivism_by_detector": report::recidivism_by_detector(&recent),
            "mean_resolution_days": report::mean_resolution_days(&recent.resolved_by_detector),
        },
        "all_time": totals,
        "branch": branch.clone(),
        "stale_findings": stale_entries(
            &store::load_fingerprints(root, &branch), hub::now(), &triage::ignored_keys(&triaged))
            .into_iter()
            .map(|(pillar, _, label, days)| json!({
                "pillar": pillar, "finding": label, "days_unresolved": days
            }))
            .collect::<Vec<_>>(),
        "triaged": triaged.values().collect::<Vec<_>>(),
    })
}

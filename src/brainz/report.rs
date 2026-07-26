//! Derived, read-time metrics for `brainz_report`. Pure functions over the
//! persisted [`Totals`] — no I/O, no event recording. These turn the raw
//! detector-granular aggregates into the developer-effectiveness signals
//! (precision, time-to-resolution, gate funnel, search health) the report
//! surfaces, so the storage layer stays a plain ledger.

use super::events::{OutcomeKey, Resolved, Totals};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const DAY: f64 = 86_400.0;

/// Round to two decimals for a stable, human-readable JSON number.
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Mean time-to-resolution in days, per detector plus an `_overall` rollup.
/// Empty when nothing has resolved yet. Derived from summed seconds ÷ count: a
/// proxy for how actionable a detector's findings are (fast fix vs. friction).
pub fn mean_resolution_days(resolved: &BTreeMap<String, Resolved>) -> Value {
    let mut out = serde_json::Map::new();
    let (mut total_secs, mut total_count) = (0u64, 0u64);
    for (detector, r) in resolved {
        if r.count == 0 {
            continue;
        }
        total_secs += r.secs_total;
        total_count += r.count;
        out.insert(
            detector.clone(),
            json!(round2(r.secs_total as f64 / r.count as f64 / DAY)),
        );
    }
    if total_count > 0 {
        out.insert(
            "_overall".into(),
            json!(round2(total_secs as f64 / total_count as f64 / DAY)),
        );
    }
    Value::Object(out)
}

/// Detection precision per detector: of the findings that were *adjudicated*,
/// the share that were real. Real = resolved (a fix made it vanish) + debt (the
/// user confirmed it real but deferred); wrong = false_positive. Only detectors
/// with at least one adjudication appear, each with its evidence counts so a
/// low-sample precision is not mistaken for a confident one.
pub fn precision_by_detector(totals: &Totals) -> Value {
    // Collect every detector that has any evidence (a resolve or a verdict).
    let mut detectors: BTreeSet<&str> = totals
        .resolved_by_detector
        .keys()
        .map(String::as_str)
        .collect();
    for key in totals.outcomes.keys() {
        detectors.insert(&key.detector);
    }

    let mut out = serde_json::Map::new();
    for detector in detectors {
        let resolved = totals
            .resolved_by_detector
            .get(detector)
            .map_or(0, |r| r.count);
        let debt = outcome(totals, "debt", detector);
        let fp = outcome(totals, "false_positive", detector);
        let real = resolved + debt;
        let adjudicated = real + fp;
        if adjudicated == 0 {
            continue;
        }
        out.insert(
            detector.to_string(),
            json!({
                "precision": round2(real as f64 / adjudicated as f64),
                "resolved": resolved,
                "debt": debt,
                "false_positive": fp,
            }),
        );
    }
    Value::Object(out)
}

/// One outcome count keyed by verdict and detector.
fn outcome(totals: &Totals, verdict: &str, detector: &str) -> u64 {
    totals
        .outcomes
        .get(&OutcomeKey::new(verdict, detector))
        .copied()
        .unwrap_or(0)
}

/// A detector is judged noisy below this precision, once it has at least
/// [`MIN_ADJUDICATED`] verdicts/fixes (so one stray false positive can't brand it).
const PRECISION_FLOOR: f64 = 0.5;
const MIN_ADJUDICATED: u64 = 4;

/// Every detector under [`PRECISION_FLOOR`] with enough evidence — the set the
/// ranker demotes so trusted findings lead (the rule still fires).
pub fn low_precision_detectors(totals: &Totals) -> BTreeSet<String> {
    let mut detectors: BTreeSet<&str> = totals
        .resolved_by_detector
        .keys()
        .map(String::as_str)
        .collect();
    for key in totals.outcomes.keys() {
        detectors.insert(&key.detector);
    }
    detectors
        .into_iter()
        .filter(|d| {
            let real = totals.resolved_by_detector.get(*d).map_or(0, |r| r.count)
                + outcome(totals, "debt", d);
            let adjudicated = real + outcome(totals, "false_positive", d);
            adjudicated >= MIN_ADJUDICATED && (real as f64 / adjudicated as f64) < PRECISION_FLOOR
        })
        .map(str::to_string)
        .collect()
}

/// Evidence-backed config-hygiene suggestions for the human to act on (sensez
/// never edits its own config): noisy detectors worth tuning/excluding, hotspot
/// detectors whose fixes keep regressing, and boundary rules that never fire.
pub fn calibration_suggestions(totals: &Totals, config: &crate::config::model::Config) -> Value {
    let mut tips = Vec::new();
    for detector in low_precision_detectors(totals) {
        let fp = outcome(totals, "false_positive", &detector);
        tips.push(format!(
            "{detector}: low precision ({fp} false-positive verdict(s)) — consider tuning the \
             threshold or adding an exclude glob"
        ));
    }
    for (detector, reintro) in &totals.reintroduced_by_detector {
        if reintro.count >= 2 {
            tips.push(format!(
                "{detector}: {} fix(es) regressed — likely a hotspot; a stronger fix or a guard \
                 may stick better than re-reporting",
                reintro.count
            ));
        }
    }
    if !config.boundaries.forbidden.is_empty()
        && totals.scans > 5
        && !totals.reported_by_detector.contains_key("boundaries")
    {
        tips.push(
            "boundary rules are configured but have never fired in any scan — confirm their \
             `from`/`to` still match real modules (a typo silently never matches)"
                .to_string(),
        );
    }
    Value::Array(tips.into_iter().map(Value::String).collect())
}

/// Gate funnel: the Stop-hook gate is the enforcement point, so its share of
/// scans approximates how much analysis happens at edit time (catching issues
/// as they are introduced) versus on explicit, after-the-fact `noze_sniff` calls.
pub fn gate_funnel(totals: &Totals) -> Value {
    let by_origin = &totals.scans_by_origin;
    let total: u64 = by_origin.values().sum();
    json!({
        "by_origin": by_origin,
        "gate_share": ratio(by_origin.get("gate").copied().unwrap_or(0), total),
    })
}

/// Gate block→fix conversion: of the fingerprints the gate flagged at edit
/// time, how many are gone by the latest full scan (fixed) vs. still `open`
/// (escaped to a later commit). `open` is the key set of the most recent full
/// baseline; a blocked key absent from it counts as resolved.
///
/// `has_baseline` prevents a false success rate: until a full background
/// snapshot has been recorded, `open` is empty and every block would look
/// resolved. In that case the rate is `null`, not a false 1.0.
pub fn gate_conversion(
    blocked: &BTreeSet<String>,
    open: &BTreeSet<String>,
    has_baseline: bool,
) -> Value {
    let total = blocked.len() as u64;
    if total == 0 {
        return json!({ "blocked_findings": 0 });
    }
    if !has_baseline {
        return json!({
            "blocked_findings": total,
            "conversion_rate": Value::Null,
            "note": "no full baseline recorded yet — run noze_sniff once so resolved vs. escaped can be measured",
        });
    }
    let escaped = blocked.iter().filter(|k| open.contains(*k)).count() as u64;
    json!({
        "blocked_findings": total,
        "still_open": escaped,
        "resolved": total - escaped,
        "conversion_rate": ratio(total - escaped, total),
    })
}

/// Search effectiveness: the share of doc searches that returned nothing. A
/// rising zero-hit rate means the eyez index is missing what developers ask
/// for (undocumented code, or queries it cannot match).
pub fn search_health(totals: &Totals) -> Value {
    json!({
        "searches": totals.searches,
        "zero_hit": totals.searches_zero_hit,
        "zero_hit_rate": ratio(totals.searches_zero_hit, totals.searches),
    })
}

/// `numerator / denominator` rounded to two decimals; 0.0 when nothing yet.
fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        round2(numerator as f64 / denominator as f64)
    }
}

/// Fix reintroductions per detector: findings that were fixed and later came
/// back. Each entry pairs the reintroduction count with how many fixes that
/// detector saw, so `rate` reads as "share of fixes that did not stick" — a
/// hotspot signal that the rule isn't landing.
pub fn fix_reintroductions_by_detector(totals: &Totals) -> Value {
    let mut out = serde_json::Map::new();
    for (detector, reintro) in &totals.reintroduced_by_detector {
        if reintro.count == 0 {
            continue;
        }
        let resolved = totals
            .resolved_by_detector
            .get(detector)
            .map_or(0, |r| r.count);
        out.insert(
            detector.clone(),
            json!({
                "reintroduced": reintro.count,
                "resolved": resolved,
                "rate": ratio(reintro.count, resolved),
                // Mean days a fix held before the finding came back.
                "mean_days_until_reintroduced": round2(reintro.secs_total as f64 / reintro.count as f64 / DAY),
            }),
        );
    }
    Value::Object(out)
}

/// Scan-throughput self-health: mean wall-time per scan and per 1000 files. A
/// rising trend flags a performance regression in sensez itself, surfaced in the
/// same ledger as the findings it produces.
pub fn self_health(totals: &Totals) -> Value {
    json!({
        "scans": totals.scans,
        "mean_scan_ms": ratio(totals.scan_ms_total, totals.scans),
        "ms_per_kfile": ratio(totals.scan_ms_total * 1000, totals.scan_files_total),
        "ms_per_kloc": ratio(totals.scan_ms_total * 1000, totals.scan_loc_total),
        "mean_files": ratio(totals.scan_files_total, totals.scans),
    })
}

/// Config pressure: how often the effective config changed between scans, plus
/// the current scope knobs. Each exclusion glob is an implicit verdict on what
/// Sensez should not report, so churn here is signal — not noise — about trust.
pub fn config_pressure(totals: &Totals, config: &crate::config::model::Config) -> Value {
    json!({
        "changes": totals.config_changes,
        "scope": {
            "exclude_globs": config.exclude.len(),
            "duplication_exclude_globs": config.duplication.exclude.len(),
            "boundary_rules": config.boundaries.forbidden.len(),
            "duplication_threshold": config.duplication.threshold,
        },
    })
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;

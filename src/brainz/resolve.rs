//! Stable finding fingerprints + "resolved since last scan" diffing + aging.
//!
//! A fingerprint is built only from a finding's *identity* (symbols, modules,
//! rules, participating files) — never line numbers or measured metrics — so
//! unrelated edits that shift code around don't masquerade as fixes. A finding
//! counts as resolved only when its identity leaves the report entirely.
//! Each fingerprint carries a human-readable label so stale findings can be
//! shown to (and triaged by) the user.

use super::events::Resolved;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};

/// One scan's findings: pillar → fingerprints in report order.
pub type Prints = BTreeMap<String, Vec<Print>>;

/// A single finding's stable identity for a scan: a content hash, a
/// human-readable label, and the detector that produced it (`pillar/<kind>`,
/// or just the pillar for detectors without sub-kinds).
#[derive(Debug, Clone)]
pub struct Print {
    pub hash: u64,
    pub label: String,
    pub detector: String,
}

/// A fingerprint persisted across scans with its age, label, and detector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgedEntry {
    pub first_seen: u64,
    pub label: String,
    /// Detector id (`pillar/<kind>`, or just the pillar).
    pub detector: String,
}

/// Fingerprints banked as resolved, kept so a finding that later comes back is
/// detected as a *reintroduction* (a fix that did not stick). Key = fingerprint
/// hex. Pruned by age so the set cannot grow without bound.
pub type ResolvedHistory = BTreeMap<String, ResolvedRecord>;

/// One previously-resolved finding: its detector (to attribute the reintro) and
/// when it was banked as resolved (to expire stale history).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRecord {
    pub detector: String,
    pub resolved_ts: u64,
}

/// A finding can be reintroduced within this window of being resolved; after
/// it, the resolved record expires (a months-later recurrence is noise, and the
/// set must stay bounded). 30 days.
const REINTRO_WINDOW_SECS: u64 = 30 * 86_400;

/// Everything `age` derives from one scan: the carried-forward fingerprints,
/// the resolved/reintroduced tallies, and the updated resolved-history.
/// (Staleness is reported separately by [`stale_entries`] over the aged map.)
pub struct Aging {
    pub aged: Aged,
    pub resolved: BTreeMap<String, Resolved>,
    /// Detector → reintroductions. `secs_total` sums the interval each finding
    /// stayed resolved before coming back (a mirror of time-to-resolution).
    pub reintroduced: BTreeMap<String, Resolved>,
    pub history: ResolvedHistory,
}

/// Pillar → fingerprint (hex string; JSON keys must be strings) → entry.
/// Persisted as `last-scan.json`.
pub type Aged = BTreeMap<String, BTreeMap<String, AgedEntry>>;

/// Findings unresolved for this long are "stale": probable false positives or
/// accepted debt — candidates for human triage.
pub const STALE_AFTER_SECS: u64 = 7 * 86_400;

/// Fingerprint every pillar of a JSON-serialized `AnalysisReport`.
pub fn fingerprints(report: &Value) -> Prints {
    PILLARS
        .iter()
        .map(|(pillar, key_fn)| {
            let prints = report
                .get(*pillar)
                .and_then(Value::as_array)
                .map(|findings| {
                    findings
                        .iter()
                        .map(|finding| {
                            let (hash, label, sub) = key_fn(finding);
                            let detector = match sub {
                                Some(kind) if !kind.is_empty() => format!("{pillar}/{kind}"),
                                _ => pillar.to_string(),
                            };
                            Print {
                                hash,
                                label,
                                detector,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            (pillar.to_string(), prints)
        })
        .collect()
}

/// Per-detector reported counts for a report (e.g. `smells/god_module → 4`).
/// Recorded on every scan, including diff/gate scans that skip aging.
pub fn detector_counts(report: &Value) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for prints in fingerprints(report).values() {
        for print in prints {
            *counts.entry(print.detector.clone()).or_default() += 1;
        }
    }
    counts
}

/// Diff this scan against the previous one and the resolved-history. Resolved
/// findings are keyed by detector and carry their time-to-resolution; stale
/// counts stay pillar-keyed (for triage prompts). A finding present now whose
/// fingerprint is in `history` is a reintroduction (it was banked resolved and
/// came back). `history` is updated: newly-resolved fingerprints are added,
/// reappearing ones removed, and entries past [`REINTRO_WINDOW_SECS`] expired.
pub fn age(
    prev: &Aged,
    current: &Prints,
    history: &ResolvedHistory,
    now: u64,
    ignore: &HashSet<String>,
) -> Aging {
    let mut aged = Aged::new();
    let mut resolved: BTreeMap<String, Resolved> = BTreeMap::new();
    let mut reintroduced: BTreeMap<String, Resolved> = BTreeMap::new();
    // Expire records too old to count as a reintroduction, then mutate as we go.
    let mut history: ResolvedHistory = history
        .iter()
        .filter(|(_, r)| now.saturating_sub(r.resolved_ts) <= REINTRO_WINDOW_SECS)
        .map(|(k, r)| (k.clone(), r.clone()))
        .collect();
    for (pillar, prints) in current {
        let old = prev.get(pillar);
        let mut entry = BTreeMap::new();
        for print in prints {
            let key = format!("{:x}", print.hash);
            // A previously-resolved fingerprint that is present again: the fix
            // did not hold. Count it (with how long it stayed dead) and drop it
            // from history (active once more).
            if let Some(rec) = history.remove(&key) {
                let tally = reintroduced.entry(print.detector.clone()).or_default();
                tally.count += 1;
                tally.secs_total += now.saturating_sub(rec.resolved_ts);
            }
            let first_seen = old
                .and_then(|m| m.get(&key))
                .map(|e| e.first_seen)
                .unwrap_or(now);
            entry.insert(
                key,
                AgedEntry {
                    first_seen,
                    label: print.label.clone(),
                    detector: print.detector.clone(),
                },
            );
        }
        if let Some(old) = old {
            for (key, gone) in old {
                if entry.contains_key(key) || ignore.contains(key) {
                    continue;
                }
                let tally = resolved.entry(gone.detector.clone()).or_default();
                tally.count += 1;
                tally.secs_total += now.saturating_sub(gone.first_seen);
                // Bank it so a later reappearance is caught as a reintroduction.
                history.insert(
                    key.clone(),
                    ResolvedRecord {
                        detector: gone.detector.clone(),
                        resolved_ts: now,
                    },
                );
            }
        }
        aged.insert(pillar.clone(), entry);
    }
    Aging {
        aged,
        resolved,
        reintroduced,
        history,
    }
}

/// Untriaged stale findings, labeled for human review:
/// `(pillar, fingerprint key, label, days unresolved)`.
pub fn stale_entries(
    aged: &Aged,
    now: u64,
    ignore: &HashSet<String>,
) -> Vec<(String, String, String, u64)> {
    let mut out = Vec::new();
    for (pillar, prints) in aged {
        for (key, entry) in prints {
            let age = now.saturating_sub(entry.first_seen);
            if age > STALE_AFTER_SECS && !ignore.contains(key) {
                out.push((
                    pillar.clone(),
                    key.clone(),
                    entry.label.clone(),
                    age / 86_400,
                ));
            }
        }
    }
    out.sort_by_key(|entry| std::cmp::Reverse(entry.3));
    out
}

/// `(content hash, human label, detector sub-kind)`. `None` sub-kind means the
/// pillar has no finer detector (the detector id is then just the pillar).
type KeyFn = fn(&Value) -> (u64, String, Option<String>);

const PILLARS: [(&str, KeyFn); 5] = [
    ("cycles", cycle_key),
    ("dead_code", dead_code_key),
    ("boundaries", boundary_key),
    ("duplication", clone_key),
    ("smells", smell_key),
];

fn hash_parts(parts: &[&str]) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    for part in parts {
        part.hash(&mut hasher);
    }
    hasher.finish()
}

fn field<'v>(finding: &'v Value, key: &str) -> &'v str {
    finding.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// Identity: the set of modules in the cycle (order-independent).
fn cycle_key(finding: &Value) -> (u64, String, Option<String>) {
    let mut modules: Vec<&str> = finding
        .get("modules")
        .and_then(Value::as_array)
        .map(|m| m.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    modules.sort_unstable();
    (hash_parts(&modules), modules.join(" ↔ "), None)
}

/// Identity: which symbol in which module (confidence may legitimately change).
/// Detector sub-kind is the symbol kind (function / class / method / import).
fn dead_code_key(finding: &Value) -> (u64, String, Option<String>) {
    let (module, symbol, kind) = (
        field(finding, "module"),
        field(finding, "symbol"),
        field(finding, "kind"),
    );
    (
        hash_parts(&[module, symbol, kind]),
        format!("{module}::{symbol} ({kind})"),
        Some(kind.to_string()),
    )
}

/// Identity: the forbidden edge and the rule that forbids it.
fn boundary_key(finding: &Value) -> (u64, String, Option<String>) {
    let (from, to, rule) = (
        field(finding, "from_module"),
        field(finding, "to_module"),
        field(finding, "rule"),
    );
    (
        hash_parts(&[from, to, rule]),
        format!("{from} → {to} ({rule})"),
        None,
    )
}

/// Identity: the sorted set of files the clone class spans plus its arity.
/// Rows shift on every edit, so they are deliberately excluded.
fn clone_key(finding: &Value) -> (u64, String, Option<String>) {
    let mut files: Vec<&str> = finding
        .get("occurrences")
        .and_then(Value::as_array)
        .map(|occ| occ.iter().map(|o| field(o, "file")).collect())
        .unwrap_or_default();
    let arity = files.len().to_string();
    files.sort_unstable();
    files.dedup();
    let label = format!("clone x{arity}: {}", files.join(" + "));
    files.push(&arity);
    (hash_parts(&files), label, None)
}

/// Identity: smell kind on a symbol in a file (metric value may fluctuate).
/// Detector sub-kind is the smell kind (long_function / god_module / …).
fn smell_key(finding: &Value) -> (u64, String, Option<String>) {
    let (kind, file, symbol) = (
        field(finding, "kind"),
        field(finding, "file"),
        field(finding, "symbol"),
    );
    (
        hash_parts(&[kind, file, symbol]),
        format!("{kind} @ {file}::{symbol}"),
        Some(kind.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn age_tracks_resolved_staleness_and_triage_exclusions() {
        let first = fingerprints(&json!({
            "dead_code": [
                {"module": "a", "symbol": "f", "kind": "function", "line": 10},
                {"module": "a", "symbol": "g", "kind": "function", "line": 20},
            ],
            "smells": [{"kind": "long_function", "file": "a.py", "symbol": "f", "metric": 80}],
        }));
        let none = HashSet::new();
        let empty_hist = ResolvedHistory::new();
        let first_age = age(&Aged::new(), &first, &empty_hist, 1_000, &none);
        assert!(first_age.resolved.is_empty(), "first scan: nothing to diff");

        // Much later: `g` was deleted; `f` survives (moved lines, metric drifted).
        let later = 1_000 + STALE_AFTER_SECS + 86_401;
        let current = fingerprints(&json!({
            "dead_code": [{"module": "a", "symbol": "f", "kind": "function", "line": 99}],
            "smells": [{"kind": "long_function", "file": "a.py", "symbol": "f", "metric": 75}],
        }));
        let a = age(&first_age.aged, &current, &first_age.history, later, &none);
        let dead = a.resolved.get("dead_code/function").expect("g resolved");
        assert_eq!(dead.count, 1);
        assert_eq!(
            dead.secs_total,
            later - 1_000,
            "time-to-resolution = now - first_seen"
        );
        assert_eq!(
            a.resolved.get("smells/long_function"),
            None,
            "line/metric drift is not a fix"
        );
        // `f` survived from t=1000 past the stale window → reported as stale.
        let entries = stale_entries(&a.aged, later, &none);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|(p, _, label, days)| p == "dead_code"
            && label == "a::f (function)"
            && *days >= 8));

        // Human triages `f` as a false positive: it stops being stale, and its
        // later disappearance must not count as resolved value.
        let fp_key = entries
            .iter()
            .find(|(p, ..)| p == "dead_code")
            .map(|(_, k, ..)| k.clone())
            .unwrap();
        let ignore: HashSet<String> = [fp_key].into();
        assert_eq!(stale_entries(&a.aged, later, &ignore).len(), 1);
        let empty = fingerprints(&json!({"dead_code": [], "smells": []}));
        let b = age(&a.aged, &empty, &a.history, later + 1, &ignore);
        assert_eq!(
            b.resolved.get("dead_code/function"),
            None,
            "vanished FP is not value"
        );
        assert_eq!(
            b.resolved.get("smells/long_function").map(|r| r.count),
            Some(1),
            "real finding fixed counts"
        );
    }

    #[test]
    fn reintroduced_finding_is_detected_and_history_expires() {
        let none = HashSet::new();
        let dc = |line| {
            fingerprints(&json!({
                "dead_code": [{"module": "a", "symbol": "f", "kind": "function", "line": line}],
            }))
        };
        // Seen at t=100, fixed (gone) at t=200 → banked in history.
        let seen = age(&Aged::new(), &dc(10), &ResolvedHistory::new(), 100, &none);
        let gone = age(
            &seen.aged,
            &fingerprints(&json!({"dead_code": []})),
            &seen.history,
            200,
            &none,
        );
        assert_eq!(gone.resolved["dead_code/function"].count, 1);
        assert_eq!(gone.history.len(), 1, "resolved fingerprint is remembered");

        // Comes back at t=300 → reintroduction (dead for 300-200=100s), and it
        // leaves the history.
        let back = age(&gone.aged, &dc(42), &gone.history, 300, &none);
        let r = back
            .reintroduced
            .get("dead_code/function")
            .expect("reintro");
        assert_eq!((r.count, r.secs_total), (1, 100));
        assert!(back.history.is_empty(), "reappearance clears the record");

        // A reappearance beyond the window is NOT a reintroduction (record expired).
        let stale_back = age(
            &gone.aged,
            &dc(42),
            &gone.history,
            200 + REINTRO_WINDOW_SECS + 1,
            &none,
        );
        assert!(
            stale_back.reintroduced.is_empty(),
            "expired record is forgotten"
        );
    }

    #[test]
    fn cycle_identity_ignores_module_order() {
        let a = cycle_key(&json!({"modules": ["x", "y"]}));
        let b = cycle_key(&json!({"modules": ["y", "x"]}));
        assert_eq!(a.0, b.0);
        assert_eq!(a.2, None, "cycles have no detector sub-kind");
    }
}

//! Cross-scan aging: carry fingerprints forward, count resolved and
//! reintroduced findings, and maintain the resolved-history bank.

use super::events::Resolved;
use super::fingerprint::{Aged, AgedEntry, Prints, ResolvedHistory, ResolvedRecord};
use std::collections::{BTreeMap, HashSet};

/// A finding can be reintroduced within this window of being resolved; after
/// it, the resolved record expires (a months-later reappearance is noise, and the
/// set must stay bounded). 30 days.
pub const REINTRO_WINDOW_SECS: u64 = 30 * 86_400;

/// Everything `age` derives from one scan: the carried-forward fingerprints,
/// the resolved/reintroduced tallies, and the updated resolved-history.
/// (Staleness is reported separately by [`super::staleness::stale_entries`]
/// over the aged map.)
pub struct Aging {
    pub aged: Aged,
    pub resolved: BTreeMap<String, Resolved>,
    /// Detector → reintroductions. `secs_total` sums the interval each finding
    /// stayed resolved before coming back (a mirror of time-to-resolution).
    pub reintroduced: BTreeMap<String, Resolved>,
    pub history: ResolvedHistory,
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
    let mut active_history: ResolvedHistory = history
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
            if let Some(rec) = active_history.remove(&key) {
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
                active_history.insert(
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
        history: active_history,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brainz::fingerprint::{fingerprints, ResolvedHistory};
    use crate::brainz::staleness::STALE_AFTER_SECS;
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
        let entries = crate::brainz::staleness::stale_entries(&a.aged, later, &none);
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
        assert_eq!(
            crate::brainz::staleness::stale_entries(&a.aged, later, &ignore).len(),
            1
        );
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
}

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
            let key = print.key();
            // A previously-resolved fingerprint that is present again: the fix
            // did not hold. Count it (with how long it stayed dead) and drop it
            // from history (active once more).
            if let Some(rec) = active_history.remove(&key) {
                let tally = reintroduced.entry(print.class.to_string()).or_default();
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
                    class: print.class.clone(),
                },
            );
        }
        if let Some(old) = old {
            for (key, gone) in old {
                if entry.contains_key(key) || ignore.contains(key) {
                    continue;
                }
                let tally = resolved.entry(gone.class.to_string()).or_default();
                tally.count += 1;
                tally.secs_total += now.saturating_sub(gone.first_seen);
                // Bank it so a later reappearance is caught as a reintroduction.
                active_history.insert(
                    key.clone(),
                    ResolvedRecord {
                        class: gone.class.clone(),
                        resolved_ts: now,
                    },
                );
            }
        }
        aged.insert(*pillar, entry);
    }
    Aging {
        aged,
        resolved,
        reintroduced,
        history: active_history,
    }
}

#[cfg(test)]
#[path = "aging_tests.rs"]
mod tests;

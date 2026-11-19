//! Cross-scan aging.

use super::events::Resolved;
use super::fingerprint::{Aged, AgedEntry, Prints, ResolvedHistory, ResolvedRecord};
use std::collections::{BTreeMap, HashSet};

/// A finding can be reintroduced within this window of being resolved.
pub const REINTRO_WINDOW_SECS: u64 = 30 * 86_400;

/// Everything `age` derives from one scan.
pub struct Aging {
    pub aged: Aged,
    pub resolved: BTreeMap<String, Resolved>,
    /// Detector → reintroductions.
    pub reintroduced: BTreeMap<String, Resolved>,
    pub history: ResolvedHistory,
}

/// Diff this scan against the previous one and the resolved-history.
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

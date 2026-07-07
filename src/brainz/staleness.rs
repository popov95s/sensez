//! Staleness: which carried-forward findings are old enough to suspect as
//! probable false positives or accepted debt — the candidates for human
//! triage.

use super::fingerprint::Aged;
use std::collections::HashSet;

/// Findings unresolved for this long are "stale": probable false positives or
/// accepted debt — candidates for human triage.
pub const STALE_AFTER_SECS: u64 = 7 * 86_400;

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
                    pillar.to_string(),
                    key.clone(),
                    entry.label.to_string(),
                    age / 86_400,
                ));
            }
        }
    }
    out.sort_by_key(|entry| std::cmp::Reverse(entry.3));
    out
}

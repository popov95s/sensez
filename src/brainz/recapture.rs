//! Automatic fix recapture: re-derive "resolved" findings without requiring
//! the agent to report back. For each repo scanned this session whose source
//! files changed since its baseline, quietly re-run the pipeline and bank
//! findings that vanished. Runs on the periodic flush tick and at shutdown.

use super::events::Event;
use super::{aging, fingerprint, hub, store};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

/// Skip auto-rescans for repos whose original scan was slower than this —
/// shutdown must stay fast (MCP clients SIGKILL slow servers).
const MAX_RESCAN_MS: u64 = 3_000;

/// Repos whose scan takes at least this long are rescanned at most once per
/// [`SLOW_RESCAN_INTERVAL_SECS`], since each rescan repeats the whole scan.
const SLOW_SCAN_MS: u64 = 1_000;
const SLOW_RESCAN_INTERVAL_SECS: u64 = 300;

/// Re-derive resolved findings for every changed, cheap-to-scan repo.
pub(super) fn run() {
    static RECAPTURE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = RECAPTURE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    for (root, base) in hub::baselines() {
        if !root.is_dir() {
            continue;
        }
        if base.ms > MAX_RESCAN_MS
            || !rescan_due(&base, hub::now())
            || !changed_since(&root, base.ts)
            || !super::recapture_state::changed(&root, &base)
        {
            continue;
        }
        // Don't cross-diff: only recapture if still on the baseline's branch.
        if hub::branch_key(&root).as_deref() != Some(base.branch.as_str()) {
            continue;
        }
        if branch_changed_since_baseline(&root, &base.branch, base.ts) {
            hub::mark_rescanned(&root, hub::now());
            continue;
        }
        recapture(&root, &base.branch, base.threshold, base.ts);
    }
}

/// Whether the last scan or rescan of a repo is old enough for another rescan.
fn rescan_due(base: &hub::Baseline, now: u64) -> bool {
    base.ms < SLOW_SCAN_MS || now.saturating_sub(base.ts) >= SLOW_RESCAN_INTERVAL_SECS
}

fn branch_changed_since_baseline(root: &Path, branch: &str, baseline_ts: u64) -> bool {
    store::branch_updated(root, branch).is_some_and(|updated| updated != baseline_ts)
}

fn recapture(root: &Path, branch: &str, threshold: Option<usize>, baseline_ts: u64) {
    if branch_changed_since_baseline(root, branch, baseline_ts) {
        hub::mark_rescanned(root, hub::now());
        return;
    }
    let report = match crate::analyze_path_in_service(root, threshold) {
        Ok((report, _)) => report,
        Err(err) => {
            eprintln!("[sensez metrics] auto-rescan {}: {err:#}", root.display());
            return;
        }
    };
    let Ok(json) = serde_json::to_value(&report) else {
        return;
    };
    if branch_changed_since_baseline(root, branch, baseline_ts) {
        hub::mark_rescanned(root, hub::now());
        return;
    }
    let previous = store::load_fingerprints(root, branch);
    if previous.is_empty() {
        return;
    }
    let current = fingerprint::fingerprints(&json);
    let history = store::load_resolved_history(root, branch);
    let ignore = super::triage::ignored_keys(&super::triage::load(root));
    let now = hub::now();
    let aging = aging::age(&previous, &current, &history, now, &ignore);
    let write = store::BranchWrite {
        branch,
        prints: aging.aged,
        history: aging.history,
        now,
    };
    match store::save_fingerprints_if_current(root, write, baseline_ts) {
        Ok(true) => {}
        Ok(false) => {
            hub::mark_rescanned(root, hub::now());
            return;
        }
        Err(err) => {
            eprintln!("[sensez metrics] saving fingerprints: {err:#}");
            return;
        }
    }
    hub::mark_rescanned(root, now);
    let (resolved, reintroduced) = (aging.resolved, aging.reintroduced);
    if !resolved.is_empty() || !reintroduced.is_empty() {
        hub::push(root, move |session, branch| Event::AutoResolve {
            ts: now,
            session,
            branch,
            resolved,
            reintroduced,
        });
    }
}

/// Cheap stat sweep: does any discovered source file have an mtime at or
/// after `ts`? (`>=` so a same-second edit is never missed; the baseline
/// advancing after each recapture debounces repeats.)
fn changed_since(root: &Path, ts: u64) -> bool {
    let exclude = crate::config::model::Config::load(root)
        .map(|c| c.exclude)
        .unwrap_or_default();
    let files = crate::spine::crawler::discover(root, &exclude, &|p| {
        crate::profiles::registry::should_parse_path(p)
    })
    .unwrap_or_default()
    .files;
    files.iter().any(|file| {
        std::fs::metadata(file)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .is_some_and(|d| d.as_secs() >= ts)
    })
}

#[cfg(test)]
#[path = "recapture_tests.rs"]
mod tests;

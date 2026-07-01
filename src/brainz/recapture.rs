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

/// Re-derive resolved findings for every changed, cheap-to-scan repo.
pub(super) fn run() {
    static RECAPTURE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = RECAPTURE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    for (root, base) in hub::baselines() {
        if base.ms > MAX_RESCAN_MS || !changed_since(&root, base.ts) {
            continue;
        }
        // Don't cross-diff: only recapture if still on the baseline's branch.
        if hub::branch_key(&root) != base.branch {
            continue;
        }
        if branch_changed_since_baseline(&root, &base.branch, base.ts) {
            hub::mark_rescanned(&root);
            continue;
        }
        recapture(&root, &base.branch, base.threshold, base.ts);
    }
}

fn branch_changed_since_baseline(root: &Path, branch: &str, baseline_ts: u64) -> bool {
    store::branch_updated(root, branch).is_some_and(|updated| updated != baseline_ts)
}

fn recapture(root: &Path, branch: &str, threshold: Option<usize>, baseline_ts: u64) {
    if branch_changed_since_baseline(root, branch, baseline_ts) {
        hub::mark_rescanned(root);
        return;
    }
    let report = match crate::analyze_path(root, threshold, None) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("[sensez metrics] auto-rescan {}: {err:#}", root.display());
            return;
        }
    };
    let Ok(json) = serde_json::to_value(&report) else {
        return;
    };
    if branch_changed_since_baseline(root, branch, baseline_ts) {
        hub::mark_rescanned(root);
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
    match store::save_fingerprints_if_current(
        root,
        branch,
        baseline_ts,
        &aging.aged,
        &aging.history,
        now,
    ) {
        Ok(true) => {}
        Ok(false) => {
            hub::mark_rescanned(root);
            return;
        }
        Err(err) => {
            eprintln!("[sensez metrics] saving fingerprints: {err:#}");
            return;
        }
    }
    hub::mark_rescanned(root);
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
        crate::profiles::registry::parse_for_path(p).is_some()
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
mod tests {
    use crate::brainz::fingerprint::{Aged, ResolvedHistory};
    use serde_json::Value;
    use std::fs;

    /// Scan → fix → auto-recapture (no second scan call, no agent report):
    /// the vanished finding must be banked as resolved.
    #[test]
    fn recapture_banks_resolved_without_a_second_scan() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("app.py"),
            "def used():\n    return 1\n\ndef orphan():\n    return 2\n\nprint(used())\n",
        )
        .unwrap();

        let text = crate::scan(&root, None, crate::reporter::Format::Json, 0).unwrap();
        let report: Value = serde_json::from_str(&text).unwrap();
        crate::brainz::record_scan(
            &root,
            &report,
            std::time::Duration::from_millis(1),
            None,
            crate::brainz::Origin::Tool,
        );

        fs::write(
            root.join("app.py"),
            "def used():\n    return 1\n\nprint(used())\n",
        )
        .unwrap();
        super::run();

        let totals = crate::brainz::usage_report(&root);
        assert_eq!(
            totals["all_time"]["resolved_by_detector"]["dead_code/function"]["count"], 1,
            "auto-recapture must record the deleted orphan as resolved"
        );
    }

    #[test]
    fn stale_recapture_baseline_is_a_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("app.py"), "def orphan():\n    return 1\n").unwrap();

        let text = crate::scan(&root, None, crate::reporter::Format::Json, 0).unwrap();
        let report: Value = serde_json::from_str(&text).unwrap();
        crate::brainz::record_scan(
            &root,
            &report,
            std::time::Duration::from_millis(1),
            None,
            crate::brainz::Origin::Tool,
        );
        crate::brainz::flush();

        super::store::save_fingerprints(&root, "", &Aged::new(), &ResolvedHistory::new(), u64::MAX)
            .unwrap();

        super::recapture(&root, "", None, 1);

        let report = crate::brainz::usage_report(&root);
        assert!(
            report["all_time"]["resolved_by_detector"]
                .as_object()
                .unwrap()
                .is_empty(),
            "a stale process must not resolve from an obsolete baseline"
        );
        assert!(
            report["all_time"]["reintroduced_by_detector"]
                .as_object()
                .unwrap()
                .is_empty(),
            "a stale process must not reintroduce from an obsolete baseline"
        );
    }
}

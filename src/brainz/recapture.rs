//! Automatic fix recapture: re-derive "resolved" findings without requiring
//! the agent to report back. For each repo scanned this session whose source
//! files changed since its baseline, quietly re-run the pipeline and bank
//! findings that vanished. Runs on the periodic flush tick and at shutdown.

use super::events::Event;
use super::{hub, resolve, store};
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Skip auto-rescans for repos whose original scan was slower than this —
/// shutdown must stay fast (MCP clients SIGKILL slow servers).
const MAX_RESCAN_MS: u64 = 3_000;

/// Re-derive resolved findings for every changed, cheap-to-scan repo.
pub(super) fn run() {
    for (root, base) in hub::baselines() {
        if base.ms > MAX_RESCAN_MS || !changed_since(&root, base.ts) {
            continue;
        }
        // Don't cross-diff: only recapture if still on the baseline's branch.
        if hub::branch_key(&root) != base.branch {
            continue;
        }
        recapture(&root, &base.branch, base.threshold);
    }
}

fn recapture(root: &Path, branch: &str, threshold: Option<usize>) {
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
    let previous = store::load_fingerprints(root, branch);
    if previous.is_empty() {
        return;
    }
    let current = resolve::fingerprints(&json);
    let history = store::load_resolved_history(root, branch);
    let ignore = super::triage::ignored_keys(&super::triage::load(root));
    let aging = resolve::age(&previous, &current, &history, hub::now(), &ignore);
    if let Err(err) =
        store::save_fingerprints(root, branch, &aging.aged, &aging.history, hub::now())
    {
        eprintln!("[sensez metrics] saving fingerprints: {err:#}");
    }
    hub::mark_rescanned(root);
    let (resolved, reintroduced) = (aging.resolved, aging.reintroduced);
    if !resolved.is_empty() || !reintroduced.is_empty() {
        hub::push(root, move |session, branch| Event::AutoResolve {
            ts: hub::now(),
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
    let files = crate::spine::crawler::discover(root, &exclude)
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
        crate::brainz::record_scan(&root, &report, 1, None, crate::brainz::Origin::Tool);

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
}

//! In-memory metrics hub: per-process session state, repo caches, and event queue.

use super::events::{Event, Totals};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// A reproducible record of the last real scan of a repo.
#[derive(Clone)]
pub(super) struct Baseline {
    pub ts: u64,
    pub ms: u64,
    pub threshold: Option<usize>,
    /// Branch the baseline was taken on. Recapture skips when the working tree
    /// has since switched branches (the re-scan would reflect different code).
    pub branch: String,
}

#[derive(Default)]
struct RepoState {
    enabled: Option<bool>,
    session: Totals,
    pending: Vec<Event>,
    baseline: Option<Baseline>,
    noisy: Option<BTreeSet<String>>,
}

struct Hub {
    session_id: String,
    started: u64,
    repos: HashMap<PathBuf, RepoState>,
}

static HUB: OnceLock<Mutex<Hub>> = OnceLock::new();

pub(super) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn hub() -> MutexGuard<'static, Hub> {
    let mutex = HUB.get_or_init(|| {
        let started = now();
        Mutex::new(Hub {
            session_id: format!("{}-{started}", std::process::id()),
            started,
            repos: HashMap::new(),
        })
    });
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub(super) fn enabled(root: &Path) -> bool {
    let mut hub = hub();
    let state = hub.repos.entry(root.to_path_buf()).or_default();
    if state.enabled.is_none() {
        state.enabled = Some(
            crate::config::model::Config::load(root)
                .map(|c| c.self_improvement.enabled)
                .unwrap_or(true),
        );
    }
    state.enabled.unwrap_or(true)
}

pub(super) fn branch_key(root: &Path) -> String {
    crate::diff::git::current_branch(root).unwrap_or_default()
}

pub(super) fn push(root: &Path, build: impl FnOnce(String, String) -> Event) {
    if !enabled(root) {
        return;
    }
    let branch = branch_key(root);
    let session = hub().session_id.clone();
    let event = build(session, branch);
    let mut hub = hub();
    let state = hub.repos.entry(root.to_path_buf()).or_default();
    state.session.absorb(&event);
    state.pending.push(event);
}

pub(super) fn set_baseline(root: &Path, baseline: Baseline) {
    hub().repos.entry(root.to_path_buf()).or_default().baseline = Some(baseline);
}

pub(super) fn baselines() -> Vec<(PathBuf, Baseline)> {
    hub()
        .repos
        .iter()
        .filter_map(|(root, state)| state.baseline.clone().map(|b| (root.clone(), b)))
        .collect()
}

pub(super) fn mark_rescanned(root: &Path) {
    if let Some(state) = hub().repos.get_mut(root) {
        if let Some(baseline) = state.baseline.as_mut() {
            baseline.ts = now();
        }
    }
}

#[cfg(feature = "eyez")]
pub(super) fn session_searches(root: &Path) -> u64 {
    hub().repos.get(root).map_or(0, |r| r.session.searches)
}

pub(super) struct SessionSnapshot {
    pub session_id: String,
    pub started: u64,
    pub totals: Totals,
}

pub(super) fn session_snapshot(root: &Path) -> SessionSnapshot {
    let hub = hub();
    SessionSnapshot {
        session_id: hub.session_id.clone(),
        started: hub.started,
        totals: hub
            .repos
            .get(root)
            .map(|r| r.session.clone())
            .unwrap_or_default(),
    }
}

pub(super) fn cached_noisy(
    root: &Path,
    compute: impl FnOnce() -> BTreeSet<String>,
) -> BTreeSet<String> {
    let mut hub = hub();
    let state = hub.repos.entry(root.to_path_buf()).or_default();
    state.noisy.get_or_insert_with(compute).clone()
}

pub(super) fn drain_pending() -> Vec<(PathBuf, Vec<Event>)> {
    let mut hub = hub();
    hub.repos
        .iter_mut()
        .filter(|(_, state)| !state.pending.is_empty())
        .map(|(root, state)| (root.clone(), std::mem::take(&mut state.pending)))
        .collect()
}

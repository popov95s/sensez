//! Per-session gate state persisted under `.sensez/gate/`, independent of metrics.

use crate::report::AnalysisReport;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const STATE_FILE: &str = "state.json";
const LOCK_FILE: &str = "state.lock";
pub(super) const SCOPE_TTL_SECS: u64 = 7 * 86_400;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct ScopeState {
    updated: u64,
    blocked: BTreeSet<String>,
    resolved: BTreeSet<String>,
    pub repeats: BTreeMap<String, super::repeats::RepeatState>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct GateState {
    scopes: BTreeMap<String, ScopeState>,
}

/// Run `apply` on the state of `scope`, then persist it.
pub(super) fn update<R>(
    root: &Path,
    scope: Option<&str>,
    now: u64,
    apply: impl FnOnce(&mut ScopeState) -> R,
) -> R {
    let dir = match crate::dotdir::ensure(root, Some("gate")) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("[sensez gate] gate state unavailable, using a fresh one: {err:#}");
            return apply(&mut ScopeState::default());
        }
    };
    let _lock = crate::brainz::file_lock::acquire_in(&dir, LOCK_FILE)
        .map_err(|err| eprintln!("[sensez gate] gate state written without a lock: {err:#}"));
    let path = dir.join(STATE_FILE);
    let mut state: GateState = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    state
        .scopes
        .retain(|_, scope| now.saturating_sub(scope.updated) < SCOPE_TTL_SECS);
    let scope_state = state
        .scopes
        .entry(scope.unwrap_or_default().to_string())
        .or_default();
    scope_state.updated = now;
    let result = apply(scope_state);
    let written = serde_json::to_vec(&state)
        .map_err(anyhow::Error::from)
        .and_then(|bytes| write_replacing(&path, &bytes));
    if let Err(err) = written {
        eprintln!("[sensez gate] saving gate state: {err:#}");
    }
    result
}

pub(super) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn write_replacing(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

impl ScopeState {
    /// Prune findings already blocked; returns identities that were blocked, fixed and are back.
    pub(super) fn recall(&mut self, report: &mut AnalysisReport) -> BTreeSet<String> {
        let current: BTreeSet<String> = crate::brainz::finding_identities(report)
            .into_keys()
            .collect();
        let fixed: Vec<String> = self.blocked.difference(&current).cloned().collect();
        for key in fixed {
            self.blocked.remove(&key);
            self.resolved.insert(key);
        }
        let reintroduced: BTreeSet<String> =
            self.resolved.intersection(&current).cloned().collect();
        self.resolved.retain(|key| !reintroduced.contains(key));
        crate::brainz::retain_unseen_gate_findings(report, &self.blocked);
        reintroduced
    }

    /// Remember every finding in `report` as blocked.
    pub(super) fn remember(&mut self, report: &AnalysisReport) {
        self.blocked
            .extend(crate::brainz::finding_identities(report).into_keys());
    }
}

#[cfg(test)]
#[path = "gate_state_tests.rs"]
mod tests;

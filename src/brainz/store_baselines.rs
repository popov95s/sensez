//! Per-branch fingerprint baselines: `baselines.json` indexes branches by last
//! update, and each branch's fingerprints and resolved-history live in their own
//! file under `baselines/`, rewritten only when they change. The single-file
//! `last-scan.json` of earlier releases is read until the first write moves it over.

use super::super::file_lock;
use super::super::fingerprint::{Aged, ResolvedHistory};
use super::{dir, json_file, write_durable};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Cap on distinct branches kept (prune oldest by `updated`). Bounds growth
/// across many short-lived feature branches.
const MAX_BRANCHES: usize = 12;

/// One branch's fingerprints and resolved-history (for reintroduction detection).
#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
struct BranchData {
    prints: Aged,
    history: ResolvedHistory,
}

/// A branch's last update; data written before per-branch files is still inline.
#[derive(Default, Serialize, Deserialize)]
struct BranchEntry {
    updated: u64,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    inline: Option<BranchData>,
}

/// `baselines.json`, or the legacy `last-scan.json` with data inline. Keying by branch stops resolved-tracking from cross-diffing
/// findings when the working tree switches branches.
#[derive(Default, Serialize, Deserialize)]
struct BranchIndex {
    branches: BTreeMap<String, BranchEntry>,
}

fn index_path(root: &Path) -> PathBuf {
    dir(root).join("baselines.json")
}

fn legacy_path(root: &Path) -> PathBuf {
    dir(root).join("last-scan.json")
}

fn branch_path(root: &Path, branch: &str) -> PathBuf {
    let name = format!("{:016x}.json", crate::fingerprints::hash_parts(&[branch]));
    dir(root).join("baselines").join(name)
}

fn load_index(root: &Path) -> BranchIndex {
    match index_path(root) {
        path if path.exists() => json_file::load_or_default(&path),
        _ => json_file::load_or_default(&legacy_path(root)),
    }
}

fn load_branch(root: &Path, index: &BranchIndex, branch: &str) -> Option<BranchData> {
    let entry = index.branches.get(branch)?;
    match &entry.inline {
        Some(inline) => Some(inline.clone()),
        None => Some(json_file::load_or_default(&branch_path(root, branch))),
    }
}

/// Last persisted update time for `branch`, if a baseline exists.
pub fn branch_updated(root: &Path, branch: &str) -> Option<u64> {
    load_index(root)
        .branches
        .get(branch)
        .map(|entry| entry.updated)
}

/// Load the fingerprints recorded by the previous full scan **on `branch`**
/// (empty when none — e.g. the first scan on a freshly checked-out branch).
pub fn load_fingerprints(root: &Path, branch: &str) -> Aged {
    branch_state(root, branch).0
}

/// One branch's fingerprints plus whether a full-scan baseline exists at all.
/// The flag lets the report distinguish "clean repo" (baseline exists, empty)
/// from "never fully scanned" (no baseline).
pub fn branch_state(root: &Path, branch: &str) -> (Aged, bool) {
    match load_branch(root, &load_index(root), branch) {
        Some(data) => (data.prints, true),
        None => (Aged::default(), false),
    }
}

/// Load the resolved-history (banked-resolved fingerprints) for `branch`, used
/// to detect findings that were fixed and later reintroduced.
pub fn load_resolved_history(root: &Path, branch: &str) -> ResolvedHistory {
    load_branch(root, &load_index(root), branch)
        .map(|data| data.history)
        .unwrap_or_default()
}

/// One branch's new baseline, stamped `now`.
pub struct BranchWrite<'a> {
    pub branch: &'a str,
    pub prints: Aged,
    pub history: ResolvedHistory,
    pub now: u64,
}

/// Persist this scan's fingerprints and resolved-history under `branch`,
/// stamping `now` and pruning to the most-recently-updated [`MAX_BRANCHES`].
#[cfg(test)]
pub fn save_fingerprints(
    root: &Path,
    branch: &str,
    prints: &Aged,
    history: &ResolvedHistory,
    now: u64,
) -> Result<()> {
    update_fingerprints(root, branch, now, |_, _| (prints.clone(), history.clone()))
}

/// Write `write` only when the branch was last updated at `expected_updated`.
pub fn save_fingerprints_if_current(
    root: &Path,
    write: BranchWrite,
    expected_updated: u64,
) -> Result<bool> {
    let _lock = file_lock::acquire(root, "last-scan.lock")?;
    let mut index = load_index(root);
    let current = index.branches.get(write.branch).map(|entry| entry.updated);
    if current != Some(expected_updated) {
        return Ok(false);
    }
    let previous = load_branch(root, &index, write.branch);
    commit(root, &mut index, write, previous)?;
    Ok(true)
}

pub fn update_fingerprints(
    root: &Path,
    branch: &str,
    now: u64,
    update: impl FnOnce(Aged, ResolvedHistory) -> (Aged, ResolvedHistory),
) -> Result<()> {
    let _lock = file_lock::acquire(root, "last-scan.lock")?;
    let mut index = load_index(root);
    let previous = load_branch(root, &index, branch);
    let BranchData { prints, history } = previous.clone().unwrap_or_default();
    let (prints, history) = update(prints, history);
    let write = BranchWrite {
        branch,
        prints,
        history,
        now,
    };
    commit(root, &mut index, write, previous)
}

/// Write the branch file when its data changed, move inline legacy data out, prune, then write the index.
fn commit(
    root: &Path,
    index: &mut BranchIndex,
    write: BranchWrite,
    previous: Option<BranchData>,
) -> Result<()> {
    let branches = crate::dotdir::ensure(root, Some("local-metrics"))?.join("baselines");
    std::fs::create_dir_all(&branches)
        .with_context(|| format!("creating {}", branches.display()))?;
    for (branch, entry) in &mut index.branches {
        if let Some(inline) = entry.inline.take() {
            write_branch(root, branch, &inline)?;
        }
    }
    let data = BranchData {
        prints: write.prints,
        history: write.history,
    };
    if previous.as_ref() != Some(&data) || !branch_path(root, write.branch).exists() {
        write_branch(root, write.branch, &data)?;
    }
    index.branches.insert(
        write.branch.to_string(),
        BranchEntry {
            updated: write.now,
            inline: None,
        },
    );
    for stale in prune_branches(index) {
        let _ = std::fs::remove_file(branch_path(root, &stale));
    }
    let json = serde_json::to_vec(index).context("serializing fingerprint index")?;
    write_durable(&index_path(root), &json)?;
    match std::fs::remove_file(legacy_path(root)) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
            Err(err).with_context(|| format!("removing {}", legacy_path(root).display()))
        }
        _ => Ok(()),
    }
}

fn write_branch(root: &Path, branch: &str, data: &BranchData) -> Result<()> {
    let json = serde_json::to_vec(data).context("serializing fingerprints")?;
    write_durable(&branch_path(root, branch), &json)
}

/// Drop branches beyond [`MAX_BRANCHES`], keeping the most recently updated; returns the dropped names.
fn prune_branches(index: &mut BranchIndex) -> Vec<String> {
    if index.branches.len() <= MAX_BRANCHES {
        return Vec::new();
    }
    let mut by_recency: Vec<(String, u64)> = index
        .branches
        .iter()
        .map(|(b, e)| (b.clone(), e.updated))
        .collect();
    by_recency.sort_by_key(|(_, updated)| *updated);
    let stale: Vec<String> = by_recency
        .into_iter()
        .take(index.branches.len() - MAX_BRANCHES)
        .map(|(branch, _)| branch)
        .collect();
    for branch in &stale {
        index.branches.remove(branch);
    }
    stale
}

#[cfg(test)]
#[path = "store_baselines_tests.rs"]
mod tests;

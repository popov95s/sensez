//! Disk persistence for local metrics: `.sensez/local-metrics/` inside the target
//! repo (already gitignored alongside the eyez cache). Plain JSON only —
//! no network, no exporters.

use super::events::{Event, Totals};
use super::file_lock;
use super::fingerprint::{Aged, ResolvedHistory};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Cap on distinct branches kept in `last-scan.json` (prune oldest by
/// `updated`). Bounds growth across many short-lived feature branches.
const MAX_BRANCHES: usize = 12;

pub(super) fn dir(root: &Path) -> PathBuf {
    root.join(".sensez").join("local-metrics")
}

/// Load the repo's all-time aggregates (default-empty when missing/corrupt —
/// metrics must never fail the server).
pub fn load_totals(root: &Path) -> Totals {
    fs::read(dir(root).join("totals.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Atomically replace the repo's all-time aggregates.
pub fn save_totals(root: &Path, totals: &Totals) -> Result<()> {
    let d = crate::dotdir::ensure(root, Some("local-metrics"))?;
    let json = serde_json::to_vec_pretty(totals).context("serializing totals")?;
    write_durable(&d.join("totals.json"), &json)
}

pub(super) fn write_durable(target: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let tmp = target.with_extension("tmp");
    let mut file = fs::File::create(&tmp)
        .with_context(|| format!("creating {}", tmp.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("writing {}", tmp.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing {}", tmp.display()))?;
    drop(file);
    if !target.parent().is_some_and(|parent| parent.exists()) {
        return Ok(());
    }
    fs::rename(&tmp, target).with_context(|| format!("replacing {}", target.display()))?;
    if let Some(parent) = target.parent() {
        if let Ok(handle) = fs::File::open(parent) {
            let _ = handle.sync_all();
        }
    }
    Ok(())
}

/// Append events to the repo's `events.jsonl` audit log.
pub fn append_events(root: &Path, events: &[Event]) -> Result<()> {
    use std::io::Write;
    if events.is_empty() {
        return Ok(());
    }
    let d = crate::dotdir::ensure(root, Some("local-metrics"))?;
    let mut lines = String::new();
    for event in events {
        lines.push_str(&serde_json::to_string(event).context("serializing event")?);
        lines.push('\n');
    }
    let path = d.join("events.jsonl");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("appending {}", path.display()))?;
    file.write_all(lines.as_bytes())
        .with_context(|| format!("appending {}", path.display()))?;
    // The log is the source of truth for resolution metrics; make the batch
    // durable before reporting success.
    file.sync_all()
        .with_context(|| format!("syncing {}", path.display()))?;
    Ok(())
}

/// `events.jsonl` is compacted once it grows past this (bytes). The append-only
/// log must stay bounded; compaction keeps only the recent retention window.
const EVENTS_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Parse the repo's event log (skipping any unparseable lines). Empty when the
/// log is missing — callers treat that as "no history yet".
pub fn load_events(root: &Path) -> Vec<Event> {
    fs::read_to_string(dir(root).join("events.jsonl"))
        .map(|text| {
            text.lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// If the event log has grown past [`EVENTS_MAX_BYTES`], rewrite it keeping only
/// events at or after `keep_after`. A no-op when the log is small or absent, so
/// it is cheap to call on every flush. Errors are returned for the caller to log.
pub fn compact_events(root: &Path, keep_after: u64) -> Result<()> {
    let path = dir(root).join("events.jsonl");
    let oversize = fs::metadata(&path)
        .map(|m| m.len() > EVENTS_MAX_BYTES)
        .unwrap_or(false);
    if !oversize {
        return Ok(());
    }
    let kept: Vec<Event> = load_events(root)
        .into_iter()
        .filter(|e| e.ts() >= keep_after)
        .collect();
    let mut text = String::new();
    for event in &kept {
        text.push_str(&serde_json::to_string(event).context("serializing event")?);
        text.push('\n');
    }
    write_durable(&path, text.as_bytes())
}

/// Per-branch fingerprint baseline with its last-updated time (for pruning).
#[derive(Default, Serialize, Deserialize)]
struct BranchEntry {
    updated: u64,
    prints: Aged,
    /// Fingerprints banked as resolved, for reintroduction detection.
    history: ResolvedHistory,
}

/// `last-scan.json`: one fingerprint baseline per branch. Keying by branch
/// stops resolved-tracking from cross-diffing findings when the working tree
/// switches branches (which legitimately yields different findings).
#[derive(Default, Serialize, Deserialize)]
struct BranchBaselines {
    branches: BTreeMap<String, BranchEntry>,
}

fn load_baselines(root: &Path) -> BranchBaselines {
    // A pre-branch (flat `Aged`) file no longer parses here and degrades to an
    // empty set — the next scan simply rebuilds that branch's baseline.
    fs::read(dir(root).join("last-scan.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Last persisted update time for `branch`, if a baseline exists.
pub fn branch_updated(root: &Path, branch: &str) -> Option<u64> {
    load_baselines(root)
        .branches
        .get(branch)
        .map(|entry| entry.updated)
}

/// Load the fingerprints recorded by the previous full scan **on `branch`**
/// (empty when none — e.g. the first scan on a freshly checked-out branch).
pub fn load_fingerprints(root: &Path, branch: &str) -> Aged {
    load_baselines(root)
        .branches
        .remove(branch)
        .map(|e| e.prints)
        .unwrap_or_default()
}

/// One branch's baseline state in a single parse of `last-scan.json`:
/// its fingerprints plus whether a full-scan baseline exists at all. The flag
/// lets the report distinguish "clean repo" (baseline exists, empty) from
/// "never fully scanned" (no baseline).
pub fn branch_state(root: &Path, branch: &str) -> (Aged, bool) {
    match load_baselines(root).branches.get(branch) {
        Some(entry) => (entry.prints.clone(), true),
        None => (Aged::default(), false),
    }
}

/// Load the resolved-history (banked-resolved fingerprints) for `branch`, used
/// to detect findings that were fixed and later reintroduced.
pub fn load_resolved_history(root: &Path, branch: &str) -> ResolvedHistory {
    load_baselines(root)
        .branches
        .remove(branch)
        .map(|e| e.history)
        .unwrap_or_default()
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

pub fn save_fingerprints_if_current(
    root: &Path,
    branch: &str,
    expected_updated: u64,
    prints: &Aged,
    history: &ResolvedHistory,
    now: u64,
) -> Result<bool> {
    let _lock = file_lock::acquire(root, "last-scan.lock")?;
    save_fingerprints_locked(root, branch, prints, history, now, Some(expected_updated))
}

pub fn update_fingerprints(
    root: &Path,
    branch: &str,
    now: u64,
    update: impl FnOnce(Aged, ResolvedHistory) -> (Aged, ResolvedHistory),
) -> Result<()> {
    let _lock = file_lock::acquire(root, "last-scan.lock")?;
    let mut all = load_baselines(root);
    let (prints, history) = match all.branches.remove(branch) {
        Some(entry) => (entry.prints, entry.history),
        None => (Aged::default(), ResolvedHistory::default()),
    };
    let (prints, history) = update(prints, history);
    commit_baselines(root, branch, prints, history, now, None).map(|_| ())
}

/// Drop branches beyond [`MAX_BRANCHES`], keeping the most recently updated.
fn prune_branches(all: &mut BranchBaselines) {
    if all.branches.len() <= MAX_BRANCHES {
        return;
    }
    let mut by_recency: Vec<(String, u64)> = all
        .branches
        .iter()
        .map(|(b, e)| (b.clone(), e.updated))
        .collect();
    by_recency.sort_by_key(|(_, updated)| *updated);
    for (stale, _) in by_recency
        .into_iter()
        .take(all.branches.len() - MAX_BRANCHES)
    {
        all.branches.remove(&stale);
    }
}

fn commit_baselines(
    root: &Path,
    branch: &str,
    prints: Aged,
    history: ResolvedHistory,
    now: u64,
    expected_updated: Option<u64>,
) -> Result<bool> {
    let d = crate::dotdir::ensure(root, Some("local-metrics"))?;
    let mut all = load_baselines(root);
    if let Some(expected) = expected_updated {
        let current = all.branches.get(branch).map(|entry| entry.updated);
        if current != Some(expected) {
            return Ok(false);
        }
    }
    all.branches.insert(
        branch.to_string(),
        BranchEntry {
            updated: now,
            prints,
            history,
        },
    );
    prune_branches(&mut all);
    let json = serde_json::to_vec(&all).context("serializing fingerprints")?;
    write_durable(&d.join("last-scan.json"), &json)?;
    Ok(true)
}

fn save_fingerprints_locked(
    root: &Path,
    branch: &str,
    prints: &Aged,
    history: &ResolvedHistory,
    now: u64,
    expected_updated: Option<u64>,
) -> Result<bool> {
    commit_baselines(
        root,
        branch,
        prints.clone(),
        history.clone(),
        now,
        expected_updated,
    )
}

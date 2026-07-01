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
use std::io::Write;
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
    let tmp = d.join("totals.json.tmp");
    fs::write(&tmp, json).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, d.join("totals.json")).context("replacing totals.json")?;
    Ok(())
}

/// Append events to the repo's `events.jsonl` audit log.
pub fn append_events(root: &Path, events: &[Event]) -> Result<()> {
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
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(lines.as_bytes()))
        .with_context(|| format!("appending {}", path.display()))?;
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
    let tmp = path.with_extension("jsonl.tmp");
    fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, &path).context("replacing events.jsonl")?;
    Ok(())
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

/// Whether a full-scan baseline has ever been recorded for `branch`. Lets the
/// report distinguish "clean repo" (baseline exists, empty) from "never fully
/// scanned" (no baseline) — they look identical through [`load_fingerprints`].
pub fn has_baseline(root: &Path, branch: &str) -> bool {
    load_baselines(root).branches.contains_key(branch)
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
pub fn save_fingerprints(
    root: &Path,
    branch: &str,
    prints: &Aged,
    history: &ResolvedHistory,
    now: u64,
) -> Result<()> {
    let _lock = file_lock::acquire(root, "last-scan.lock")?;
    save_fingerprints_locked(root, branch, prints, history, now, None).map(|_| ())
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

fn save_fingerprints_locked(
    root: &Path,
    branch: &str,
    prints: &Aged,
    history: &ResolvedHistory,
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
            prints: prints.clone(),
            history: history.clone(),
        },
    );
    if all.branches.len() > MAX_BRANCHES {
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
    let json = serde_json::to_vec(&all).context("serializing fingerprints")?;
    fs::write(d.join("last-scan.json"), json).context("writing last-scan.json")?;
    Ok(true)
}

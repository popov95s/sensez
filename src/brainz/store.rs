//! Disk persistence for local metrics: `.sensez/local-metrics/` inside the target
//! repo (already gitignored alongside the eyez cache). Plain JSON only —
//! no network, no exporters.

use super::events::{Event, Totals};
use super::json_file;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn dir(root: &Path) -> PathBuf {
    root.join(".sensez").join("local-metrics")
}

/// Load the repo's all-time aggregates (default-empty when missing/corrupt —
/// metrics must never fail the server).
pub fn load_totals(root: &Path) -> Totals {
    json_file::load_or_default(&dir(root).join("totals.json"))
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
    let mut file = fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
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

#[path = "store_baselines.rs"]
mod baselines;
pub use baselines::*;

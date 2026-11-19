//! Which workspace scans run now and which wait, bounded by scan capacity.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) const MAX_CONCURRENT_SCANS: usize = 2;

/// Scan bookkeeping for one workspace root.
pub(super) struct Workspace {
    pub(super) path: PathBuf,
    pub(super) generation: u64,
    pub(super) running: bool,
    pub(super) pending: bool,
    pub(super) published: BTreeSet<String>,
}

/// Bump `root`'s generation; the generation to scan now, or `None` when it waits as pending.
pub(super) fn mark_dirty(
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
    root: &Path,
) -> Option<u64> {
    let in_flight = running(workspaces);
    let workspace = workspaces
        .entry(root.to_path_buf())
        .or_insert_with(|| Workspace {
            path: root.to_path_buf(),
            generation: 0,
            running: false,
            pending: false,
            published: BTreeSet::new(),
        });
    workspace.generation = workspace.generation.saturating_add(1);
    if workspace.running || in_flight >= MAX_CONCURRENT_SCANS {
        workspace.pending = true;
        return None;
    }
    workspace.running = true;
    Some(workspace.generation)
}

/// The next pending workspace to start while scan capacity remains, marked running.
pub(super) fn take_pending(
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
) -> Option<(PathBuf, u64)> {
    if running(workspaces) >= MAX_CONCURRENT_SCANS {
        return None;
    }
    let workspace = workspaces
        .values_mut()
        .find(|workspace| workspace.pending && !workspace.running)?;
    workspace.pending = false;
    workspace.running = true;
    Some((workspace.path.clone(), workspace.generation))
}

fn running(workspaces: &BTreeMap<PathBuf, Workspace>) -> usize {
    workspaces
        .values()
        .filter(|workspace| workspace.running)
        .count()
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;

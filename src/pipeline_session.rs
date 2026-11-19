//! Process-local parse facts for long-lived MCP and LSP servers.

use crate::spine::cache::{ChangeStats, ParseCacheState, ProjectInputs};
use crate::spine::parser::ParseBatch;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const MAX_WORKSPACES: usize = 4;

/// Where a scan may reuse parse facts instead of parsing every file.
#[derive(Clone, Copy)]
pub(super) enum ParseCache<'a> {
    Off,
    Session(&'a AnalysisSession),
    Disk,
}

#[derive(Default)]
pub(super) struct AnalysisSession {
    states: Mutex<HashMap<PathBuf, WorkspaceEntry>>,
}

struct WorkspaceEntry {
    state: Arc<Mutex<ParseCacheState>>,
    last_used: Instant,
}

impl AnalysisSession {
    pub(super) fn parse(&self, root: &Path, project: &ProjectInputs) -> (ParseBatch, ChangeStats) {
        let full_parse = || {
            (
                crate::spine::parser::parse_sources(&project.sources),
                ChangeStats::default(),
            )
        };
        let Some(workspace) = self.workspace(root) else {
            eprintln!("[sensez] analysis cache unavailable; parsing without cache");
            return full_parse();
        };
        let Ok(mut state) = workspace.lock() else {
            eprintln!(
                "[sensez] analysis cache poisoned for {}; parsing without cache",
                root.display()
            );
            return full_parse();
        };
        let (parsed, stats) =
            crate::spine::parser::parse_sources_incremental(&project.sources, &mut state);
        state.replace(&project.sources, &parsed.files);
        (parsed, stats)
    }

    fn workspace(&self, root: &Path) -> Option<Arc<Mutex<ParseCacheState>>> {
        let Ok(mut states) = self.states.lock() else {
            return None;
        };
        if let Some(entry) = states.get_mut(root) {
            entry.last_used = Instant::now();
            return Some(Arc::clone(&entry.state));
        }
        if states.len() >= MAX_WORKSPACES {
            if let Some((lru, _)) = states
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(path, entry)| (path.clone(), entry.last_used))
            {
                states.remove(&lru);
            }
        }
        let entry = WorkspaceEntry {
            state: Arc::new(Mutex::new(ParseCacheState::default())),
            last_used: Instant::now(),
        };
        let state = Arc::clone(&entry.state);
        states.insert(root.to_path_buf(), entry);
        Some(state)
    }

    #[cfg(test)]
    pub(super) fn has_workspace(&self, root: &Path) -> bool {
        self.states
            .lock()
            .is_ok_and(|states| states.contains_key(root))
    }
}

#[cfg(test)]
#[path = "pipeline_session_tests.rs"]
mod tests;

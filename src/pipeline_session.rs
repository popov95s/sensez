//! Process-local parse facts for long-lived MCP and LSP servers.

use crate::spine::cache::{ChangeStats, ParseCacheState, ProjectInputs};
use crate::spine::parser::ParseBatch;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

const MAX_WORKSPACES: usize = 4;

#[derive(Default)]
pub(super) struct AnalysisSession {
    states: Mutex<HashMap<std::path::PathBuf, ParseCacheState>>,
}

impl AnalysisSession {
    pub(super) fn parse(&self, root: &Path, project: &ProjectInputs) -> (ParseBatch, ChangeStats) {
        let Ok(mut states) = self.states.lock() else {
            return (
                crate::spine::parser::parse_sources(&project.sources),
                ChangeStats::default(),
            );
        };
        if !states.contains_key(root) && states.len() >= MAX_WORKSPACES {
            if let Some(expired) = states.keys().next().cloned() {
                states.remove(&expired);
            }
        }
        let state = states.entry(root.to_path_buf()).or_default();
        let (parsed, stats) =
            crate::spine::parser::parse_sources_incremental(&project.sources, state);
        if parsed.issues.is_empty() {
            state.replace(&project.sources, &parsed.files);
        }
        (parsed, stats)
    }

    #[cfg(test)]
    pub(super) fn has_workspace(&self, root: &Path) -> bool {
        self.states
            .lock()
            .is_ok_and(|states| states.contains_key(root))
    }
}

#[cfg(test)]
mod tests {
    use super::AnalysisSession;

    #[test]
    fn reuses_unchanged_files_and_reparses_a_changed_file() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("module.py");
        std::fs::write(&file, "def first():\n    return 1\n").unwrap();
        let session = AnalysisSession::default();
        let first = crate::spine::cache::load_project(std::slice::from_ref(&file)).unwrap();
        assert_eq!(session.parse(root.path(), &first).1.reusable, 0);
        assert_eq!(session.parse(root.path(), &first).1.reusable, 1);

        std::fs::write(&file, "def second():\n    return 2\n").unwrap();
        let changed = crate::spine::cache::load_project(&[file]).unwrap();
        let stats = session.parse(root.path(), &changed).1;
        assert_eq!((stats.modified, stats.reusable), (1, 0));
    }
}

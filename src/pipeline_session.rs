//! Process-local parse facts for long-lived MCP and LSP servers.

use crate::spine::cache::{ChangeStats, ParseCacheState, ProjectInputs};
use crate::spine::parser::ParseBatch;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const MAX_WORKSPACES: usize = 4;

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
        if parsed.issues.is_empty() {
            state.replace(&project.sources, &parsed.files);
        }
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
mod tests {
    use super::{AnalysisSession, MAX_WORKSPACES};

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

    #[test]
    fn concurrent_workspaces_parse_independently() {
        let session = AnalysisSession::default();
        let roots: Vec<_> = (0..4)
            .map(|i| {
                let dir = tempfile::tempdir().unwrap();
                std::fs::write(
                    dir.path().join(format!("m{i}.py")),
                    format!("def f{i}():\n    return {i}\n"),
                )
                .unwrap();
                dir
            })
            .collect();
        let projects: Vec<_> = roots
            .iter()
            .map(|dir| {
                let files: Vec<_> = std::fs::read_dir(dir.path())
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .collect();
                crate::spine::cache::load_project(&files).unwrap()
            })
            .collect();
        std::thread::scope(|scope| {
            for (root, project) in roots.iter().zip(&projects) {
                scope.spawn(|| {
                    let (parsed, _) = session.parse(root.path(), project);
                    assert!(parsed.issues.is_empty());
                });
            }
        });
        for root in &roots {
            assert!(session.has_workspace(root.path()));
        }
    }

    /// Eviction follows least-recent use, not arbitrary map order.
    #[test]
    fn evicts_least_recently_used_workspace() {
        use std::thread::sleep;
        use std::time::Duration;

        let session = AnalysisSession::default();
        let dirs: Vec<_> = (0..MAX_WORKSPACES as u8 + 1)
            .map(|i| {
                let dir = tempfile::tempdir().unwrap();
                std::fs::write(
                    dir.path().join(format!("m{i}.py")),
                    format!("def f{i}():\n    return {i}\n"),
                )
                .unwrap();
                dir
            })
            .collect();
        let project_for = |dir: &tempfile::TempDir| {
            let files: Vec<_> = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            crate::spine::cache::load_project(&files).unwrap()
        };

        for dir in &dirs[..usize::from(MAX_WORKSPACES as u8)] {
            session.parse(dir.path(), &project_for(dir));
            sleep(Duration::from_millis(5));
        }
        // Touch the first workspace so the second becomes least-recently used.
        session.parse(dirs[0].path(), &project_for(&dirs[0]));
        sleep(Duration::from_millis(5));

        let newest = dirs.last().unwrap();
        session.parse(newest.path(), &project_for(newest));

        assert!(
            session.has_workspace(dirs[0].path()),
            "recently touched stays"
        );
        assert!(
            !session.has_workspace(dirs[1].path()),
            "untouched workspace is evicted"
        );
        assert!(session.has_workspace(newest.path()));
    }
}

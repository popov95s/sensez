//! Shared infrastructure for integration-style unit tests.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Write `body` to `dir/rel`, creating parent directories.
pub(crate) fn write(dir: &Path, rel: &str, body: &str) -> PathBuf {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, body).unwrap();
    path
}

pub(crate) fn metrics_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Temporary Git repository with one committed `base.py` and one test path.
pub(crate) struct GitTestRepo {
    _temp: tempfile::TempDir,
    pub(crate) root: PathBuf,
    pub(crate) file: PathBuf,
    pub(crate) path: String,
}

impl GitTestRepo {
    pub(crate) fn new(test_path: &str, base_source: &str) -> Option<Self> {
        let temp = tempfile::tempdir().ok()?;
        let root = temp.path().to_path_buf();
        if !run_git(&root, &["init"]) {
            return None;
        }
        std::fs::write(root.join("base.py"), base_source).ok()?;
        if !run_git(&root, &["add", "."]) || !commit(&root) {
            return None;
        }
        Some(Self {
            file: root.join(test_path),
            path: root.to_string_lossy().into_owned(),
            root,
            _temp: temp,
        })
    }

    pub(crate) fn importing(test_path: &str, fallback_module: &str) -> Option<Self> {
        let module = test_path.strip_suffix(".py").unwrap_or(fallback_module);
        Self::new(
            test_path,
            &format!("from {module} import live\n\nprint(live())\n"),
        )
    }

    pub(crate) fn git(&self, args: &[&str]) -> bool {
        run_git(&self.root, args)
    }

    /// Stage and commit everything in the work tree.
    pub(crate) fn commit_all(&self) -> bool {
        run_git(&self.root, &["add", "."]) && commit(&self.root)
    }
}

fn commit(root: &Path) -> bool {
    run_git(
        root,
        &[
            "-c",
            "user.email=sensez@example.test",
            "-c",
            "user.name=Sensez",
            "commit",
            "-m",
            "base",
        ],
    )
}

fn run_git(root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

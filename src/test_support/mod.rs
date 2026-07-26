//! Shared infrastructure for integration-style unit tests.

use std::path::{Path, PathBuf};
use std::process::Command;

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

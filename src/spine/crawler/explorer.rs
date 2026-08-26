//! Parallel source discovery using the `ignore` crate's threaded walker.

use crate::globs::build_globset;
use crate::report::{ScanIssue, ScanStage};
use anyhow::{anyhow, Result};
use ignore::{Error as IgnoreError, WalkBuilder, WalkState};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Result of a discovery walk: the source files found, plus how many entries
/// could not be read. A non-zero `skipped` means the scan is incomplete — it
/// is surfaced in the report metadata so it is never mistaken for "clean".
#[derive(Debug, Default)]
pub struct Discovery {
    pub files: Vec<PathBuf>,
    pub skipped: usize,
    pub issues: Vec<ScanIssue>,
}

/// Walk `root` in parallel, returning every regular file for which
/// `is_source_file` returns `true`, minus any `exclude`-glob match.
///
/// `.gitignore`, `.ignore`, and hidden-file rules are respected by default.
/// I/O errors on individual entries are counted, not silently dropped.
pub fn collect_source_files<F>(
    root: &Path,
    exclude: &[String],
    is_source_file: &F,
) -> Result<Discovery>
where
    F: Fn(&Path) -> bool + Send + Sync,
{
    if !root.exists() {
        return Err(anyhow!("path does not exist: {}", root.display()));
    }
    let excludes = build_globset(exclude)?;

    let (tx, rx) = mpsc::channel::<PathBuf>();
    let (issue_tx, issue_rx) = mpsc::channel::<ScanIssue>();
    WalkBuilder::new(root).build_parallel().run(|| {
        let tx = tx.clone();
        let issue_tx = issue_tx.clone();
        let excludes = excludes.clone();
        Box::new(move |entry| {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    let is_file = match entry.file_type() {
                        Some(ft) if ft.is_symlink() => match std::fs::metadata(path) {
                            Ok(meta) => meta.is_file(),
                            Err(err) => {
                                let _ = issue_tx.send(ScanIssue {
                                    stage: ScanStage::Discover,
                                    file: Some(path.to_path_buf()),
                                    message: format!("unreadable symlink: {err}"),
                                });
                                false
                            }
                        },
                        Some(ft) => ft.is_file(),
                        None => false,
                    };
                    if is_file && is_source_file(path) && !excludes.is_match(path) {
                        // Receiver lives until the walk completes; ignore send races.
                        let _ = tx.send(entry.into_path());
                    }
                }
                Err(err) => {
                    let _ = issue_tx.send(scan_issue_from_walk_error(&err));
                }
            }
            WalkState::Continue
        })
    });
    drop(tx);
    drop(issue_tx);

    let mut files: Vec<PathBuf> = rx.into_iter().collect();
    files.sort();
    let mut issues: Vec<ScanIssue> = issue_rx.into_iter().collect();
    issues.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.message.cmp(&b.message)));
    Ok(Discovery {
        files,
        skipped: issues.len(),
        issues,
    })
}

fn scan_issue_from_walk_error(err: &IgnoreError) -> ScanIssue {
    ScanIssue {
        stage: ScanStage::Discover,
        file: walk_error_path(err),
        message: err.to_string(),
    }
}

fn walk_error_path(err: &IgnoreError) -> Option<PathBuf> {
    match err {
        IgnoreError::WithPath { path, .. } => Some(path.clone()),
        IgnoreError::WithLineNumber { err, .. } | IgnoreError::WithDepth { err, .. } => {
            walk_error_path(err)
        }
        IgnoreError::Partial(errs) => errs.iter().find_map(walk_error_path),
        IgnoreError::Loop { child, .. } => Some(child.clone()),
        IgnoreError::Io(_)
        | IgnoreError::Glob { .. }
        | IgnoreError::UnrecognizedFileType(_)
        | IgnoreError::InvalidDefinition => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Test predicate: keep `.py` files only. Stand-in for what production
    /// callers build from `profiles::registry::parse_for_path`.
    fn is_python(path: &Path) -> bool {
        path.extension().is_some_and(|e| e == "py")
    }

    #[test]
    fn finds_py_files_and_skips_others() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("pkg")).unwrap();
        fs::write(dir.join("a.py"), "x = 1\n").unwrap();
        fs::write(dir.join("pkg/b.py"), "y = 2\n").unwrap();
        fs::write(dir.join("notes.txt"), "ignore me\n").unwrap();

        let found = collect_source_files(&dir, &[], &is_python).unwrap();
        assert_eq!(found.skipped, 0, "readable tree skips nothing");
        let names: Vec<_> = found
            .files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a.py".to_string()));
        assert!(names.contains(&"b.py".to_string()));
        assert!(!names.iter().any(|n| n.ends_with(".txt")));

        // exclude glob drops matching files from discovery
        let filtered = collect_source_files(&dir, &["**/pkg/**".to_string()], &is_python).unwrap();
        let fnames: Vec<_> = filtered
            .files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(fnames.contains(&"a.py".to_string()));
        assert!(!fnames.contains(&"b.py".to_string()), "pkg/ excluded");
    }

    #[test]
    fn missing_path_errors() {
        assert!(
            collect_source_files(Path::new("/nonexistent/sensez/xyz"), &[], &is_python).is_err()
        );
    }

    #[test]
    fn walk_errors_become_discover_issues_with_paths() {
        let err = IgnoreError::WithPath {
            path: PathBuf::from("blocked.py"),
            err: Box::new(IgnoreError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            ))),
        };

        let issue = scan_issue_from_walk_error(&err);
        assert_eq!(issue.stage, ScanStage::Discover);
        assert_eq!(issue.file, Some(PathBuf::from("blocked.py")));
        assert!(issue.message.contains("permission denied"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_sources_are_included_and_broken_links_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("real.py"), "x = 1\n").unwrap();
        std::os::unix::fs::symlink(dir.join("real.py"), dir.join("link.py")).unwrap();
        fs::create_dir(dir.join("pkg")).unwrap();
        fs::write(dir.join("pkg/nested.py"), "y = 2\n").unwrap();
        std::os::unix::fs::symlink(dir.join("pkg"), dir.join("pkg_link")).unwrap();
        std::os::unix::fs::symlink(dir.join("missing.py"), dir.join("broken.py")).unwrap();

        let found = collect_source_files(dir, &[], &is_python).unwrap();
        let names: Vec<_> = found
            .files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"real.py".to_string()));
        assert!(
            names.contains(&"link.py".to_string()),
            "file symlink must be discovered, got {names:?}"
        );
        // The directory symlink is not followed: `pkg/nested.py` is reachable
        // exactly once through the real directory.
        assert_eq!(
            names.iter().filter(|n| *n == "nested.py").count(),
            1,
            "directory symlinks must not double-count targets"
        );
        assert!(
            !names
                .iter()
                .any(|n| n.ends_with("nested.py") && n.contains("pkg_link")),
            "no path through pkg_link"
        );
        assert_eq!(
            found.issues.len(),
            1,
            "the broken symlink must be reported, not dropped silently"
        );
        assert!(found.issues[0].message.contains("unreadable symlink"));
    }
}

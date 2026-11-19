//! Parallel source discovery using the `ignore` crate's threaded walker.

use crate::globs::build_globset;
use crate::report::{ScanIssue, ScanStage};
use anyhow::{anyhow, Result};
use ignore::{Error as IgnoreError, WalkBuilder, WalkState};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Result of a discovery walk: the source files found, plus one issue per entry
/// that could not be read, surfaced in the report so an incomplete scan is never mistaken for "clean".
#[derive(Debug, Default)]
pub struct Discovery {
    pub files: Vec<PathBuf>,
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
    Ok(Discovery { files, issues })
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
#[path = "explorer_tests.rs"]
mod tests;

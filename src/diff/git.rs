//! The only subprocess use in sensez: obtain a working-tree diff from `git`.
//!
//! Isolated here so the rest of the tool stays subprocess-free. Diffs against
//! the `HEAD` commit (so staged *and* unstaged edits are seen) and treats
//! untracked source files as fully added (a freshly-written file emits no diff
//! hunks but is exactly the common edit-loop case).

use super::ChangedLines;
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use wait_timeout::ChildExt;

/// Per-invocation wall-clock cap on the `git` subprocess. Long enough for any
/// reasonable local operation (`diff`/`ls-files` on a large repo finishes in
/// well under a second), short enough that a hung `git` on a network mount
/// or a misbehaving hook does not stall the scan.
const GIT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct ChangedFiles {
    pub root: PathBuf,
    pub paths: Vec<PathBuf>,
    pub deleted: Vec<PathBuf>,
}

pub fn repository_root(scan_path: &Path) -> Result<PathBuf> {
    let root = run(&["rev-parse", "--show-toplevel"], scan_path)?;
    Ok(PathBuf::from(root.trim()))
}

/// Resolve all changed paths for test-impact analysis, including non-source
/// manifests and configuration files that may require a full-suite fallback.
pub fn changed_paths(scan_path: &Path, base: Option<&str>, staged: bool) -> Result<ChangedFiles> {
    use std::collections::BTreeSet;

    let root = repository_root(scan_path)?;
    let mut relative = BTreeSet::new();
    let mut deleted = BTreeSet::new();
    if staged {
        extend_names(
            &mut relative,
            &run(&["diff", "--cached", "--name-only", "-z"], &root)?,
        );
        extend_names(
            &mut deleted,
            &run(
                &["diff", "--cached", "--name-only", "--diff-filter=D", "-z"],
                &root,
            )?,
        );
    } else {
        if let Some(base) = base {
            validate_revision(base)?;
            let range = format!("{base}...HEAD");
            extend_names(
                &mut relative,
                &run(&["diff", "--name-only", "-z", &range], &root)?,
            );
            extend_names(
                &mut deleted,
                &run(
                    &["diff", "--name-only", "--diff-filter=D", "-z", &range],
                    &root,
                )?,
            );
        }
        extend_names(
            &mut relative,
            &run(&["diff", "--name-only", "-z", "HEAD"], &root)?,
        );
        extend_names(
            &mut deleted,
            &run(
                &["diff", "--name-only", "--diff-filter=D", "-z", "HEAD"],
                &root,
            )?,
        );
        relative.extend(untracked_relative(&root)?);
    }
    Ok(ChangedFiles {
        paths: relative.into_iter().map(|path| root.join(path)).collect(),
        deleted: deleted.into_iter().map(|path| root.join(path)).collect(),
        root,
    })
}

fn extend_names(paths: &mut std::collections::BTreeSet<PathBuf>, output: &str) {
    paths.extend(
        output
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
    );
}

fn untracked_relative(root: &Path) -> Result<Vec<PathBuf>> {
    Ok(untracked_entries(root)?
        .into_iter()
        .map(PathBuf::from)
        .collect())
}

fn untracked_entries(root: &Path) -> Result<Vec<String>> {
    let listing = run(
        &[
            "status",
            "--porcelain",
            "-z",
            "--untracked-files=all",
        ],
        root,
    )?;
    Ok(listing
        .split('\0')
        .filter(|entry| entry.starts_with("?? "))
        .map(|entry| entry["?? ".len()..].to_string())
        .filter(|path| !path.is_empty())
        .collect())
}

/// Working-tree changes vs `HEAD`, including untracked source files.
pub fn changed_vs_head(scan_path: &Path) -> Result<ChangedLines> {
    let root = run(&["rev-parse", "--show-toplevel"], scan_path)?;
    let root = Path::new(root.trim());

    let diff = run(&["diff", "--unified=0", "HEAD"], root)?;
    let mut changed = ChangedLines::from_unified(&diff, root);

    for file in untracked_sources(root)? {
        changed.add_full_file(&file);
    }
    Ok(changed)
}

fn untracked_sources(root: &Path) -> Result<Vec<PathBuf>> {
    Ok(untracked_entries(root)?
        .into_iter()
        .map(|rel| root.join(rel))
        .filter(|abs| crate::profiles::registry::parse_for_path(abs).is_some())
        .collect())
}

#[cfg(feature = "mcp")]
/// Current branch name, or `None` when not a git repo, on a detached HEAD, or
/// git is unavailable. Used to key local metrics so resolved-tracking never
/// cross-diffs findings between branches.
pub fn current_branch(path: &Path) -> Option<String> {
    let output = run_with_timeout(
        Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]),
        path,
    )
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // "HEAD" means detached; bucket those with non-git rather than guess a key.
    if name.is_empty() || name == "HEAD" {
        None
    } else {
        Some(name)
    }
}

fn validate_revision(rev: &str) -> Result<()> {
    if rev.is_empty()
        || rev.starts_with('-')
        || rev.chars().any(|ch| ch.is_whitespace() || ch.is_control())
    {
        anyhow::bail!("invalid revision '{rev}': must be a git ref without options");
    }
    Ok(())
}

fn run(args: &[&str], cwd: &Path) -> Result<String> {
    let output = run_with_timeout(Command::new("git").args(args), cwd)
        .context("failed to run `git` (is it installed and on PATH?)")?;
    if !output.status.success() {
        return Err(anyhow!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Spawn `cmd` (already configured with args + current_dir) and wait up to
/// [`GIT_TIMEOUT`] for completion. On timeout the child is killed and the
/// function returns an error rather than blocking indefinitely.
///
/// Both streams are piped AND drained concurrently on helper threads: a child
/// whose output exceeds the OS pipe buffer (~64 KB) would otherwise block on
/// write forever (it cannot exit until the pipe drains, so waiting for exit
/// first deadlocks until the timeout kills it — which then loses the diff).
fn run_with_timeout(cmd: &mut Command, cwd: &Path) -> std::io::Result<std::process::Output> {
    use std::io::Read;
    use std::process::Stdio;
    cmd.current_dir(cwd);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        })
    }
    let stdout_reader = drain(child.stdout.take());
    let stderr_reader = drain(child.stderr.take());
    match child.wait_timeout(GIT_TIMEOUT) {
        Ok(Some(status)) => {
            // The child exited, so both pipes are at EOF; the joins are prompt.
            let stdout = stdout_reader.join().unwrap_or_default();
            let stderr = stderr_reader.join().unwrap_or_default();
            Ok(std::process::Output {
                status,
                stdout,
                stderr,
            })
        }
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("`git` exceeded {GIT_TIMEOUT:?}"),
            ))
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod tests;

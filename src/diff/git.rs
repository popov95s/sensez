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
    let listing = run(&["status", "--porcelain", "--untracked-files=all"], root)?;
    Ok(listing
        .lines()
        .filter_map(|line| line.strip_prefix("?? "))
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
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
    let listing = run(&["status", "--porcelain", "--untracked-files=all"], root)?;
    Ok(listing
        .lines()
        .filter(|line| line.starts_with("?? "))
        .map(|line| line.trim_start_matches("?? ").trim())
        .filter(|rel| !rel.is_empty())
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
/// We use a `Stdio::piped` redirect for both streams so we can read them
/// *after* the child exits; if we let the child inherit the parent's stdio,
/// a child that fills its pipe would block on write and the timeout would
/// never fire (the kernel would block the child, but `wait_timeout` only
/// ticks against wall-clock).
fn run_with_timeout(cmd: &mut Command, cwd: &Path) -> std::io::Result<std::process::Output> {
    use std::io::Read;
    use std::process::Stdio;
    cmd.current_dir(cwd);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    match child.wait_timeout(GIT_TIMEOUT) {
        Ok(Some(status)) => {
            let mut stdout = Vec::new();
            if let Some(mut pipe) = child.stdout.take() {
                pipe.read_to_end(&mut stdout)?;
            }
            let mut stderr = Vec::new();
            if let Some(mut pipe) = child.stderr.take() {
                pipe.read_to_end(&mut stderr)?;
            }
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

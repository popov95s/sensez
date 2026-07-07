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
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn untracked_directory_is_expanded_to_source_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        if Command::new("git")
            .arg("init")
            .current_dir(root)
            .output()
            .is_err()
        {
            return; // git not available in this environment
        }
        let pkg = root.join("newpkg/src/deep");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("a.py"), "def f():\n    pass\n").unwrap();
        fs::write(pkg.join("b.ts"), "export const x = 1;\n").unwrap();
        fs::write(pkg.join("notes.md"), "# notes\n").unwrap();

        let found = untracked_sources(root).unwrap();
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.contains(&"a.py".to_string()),
            "nested .py expanded: {names:?}"
        );
        assert!(
            !names.contains(&"notes.md".to_string()),
            "non-source excluded"
        );
        // .ts is only recognized when the TypeScript profile is compiled in.
        #[cfg(feature = "lang-typescript")]
        assert!(
            names.contains(&"b.ts".to_string()),
            "untracked .ts included: {names:?}"
        );
    }

    #[test]
    fn diff_is_fast_with_large_gitignored_footprint() {
        use std::time::Instant;

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("git must be available")
        };
        git(&["init"]);
        git(&["config", "user.email", "test@test"]);
        git(&["config", "user.name", "test"]);

        fs::write(root.join(".gitignore"), ".venv/\nnode_modules/\n").unwrap();
        fs::write(root.join("seed.py"), "# seed\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "seed"]);

        let venv1 = root.join(".venv/lib/python3.11/site-packages");
        let venv2 = root.join("node_modules/pkg/dist");
        fs::create_dir_all(&venv1).unwrap();
        fs::create_dir_all(&venv2).unwrap();
        for i in 0..500 {
            fs::write(venv1.join(format!("mod{i}.py")), format!("# {i}\n")).unwrap();
        }
        for i in 0..500 {
            fs::write(venv2.join(format!("chunk{i}.js")), format!("// {i}\n")).unwrap();
        }

        fs::write(root.join("app.py"), "def main():\n    return 42\n").unwrap();

        let start = Instant::now();
        let changed = changed_vs_head(root).unwrap();
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_secs() < 2,
            "diff must complete in < 2 s, took {elapsed:.2?}"
        );
        assert!(
            changed.touches_file(&root.join("app.py")),
            "untracked file must appear"
        );
        assert!(
            !changed.paths().any(|p| p.starts_with(&venv1)),
            "no files from .venv should appear"
        );
        assert!(
            !changed.paths().any(|p| p.starts_with(&venv2)),
            "no files from node_modules should appear"
        );
    }
}

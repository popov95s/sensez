//! Runner adapters build ordinary commands; test collection/execution stays native.

use super::discovery::{RunnerRoot, TestFile};
use super::model::{RunMode, RunnerKind, RunnerPlan};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Full runs invoke every runner root without paths; narrowed runs pass the selected tests.
pub fn plans(
    repository: &Path,
    runners: &[RunnerRoot],
    tests: &[TestFile],
    selected: &HashSet<PathBuf>,
    mode: RunMode,
) -> Vec<RunnerPlan> {
    let mut groups: BTreeMap<(RunnerKind, PathBuf), Vec<PathBuf>> = BTreeMap::new();
    match mode {
        RunMode::Full => {
            for runner in runners {
                groups
                    .entry((runner.kind, runner.root.clone()))
                    .or_default();
            }
        }
        RunMode::Narrowed => {
            for test in tests.iter().filter(|test| selected.contains(&test.file)) {
                groups
                    .entry((test.runner, test.runner_root.clone()))
                    .or_default()
                    .push(test.file.clone());
            }
        }
    }
    groups
        .into_iter()
        .map(|((kind, root), mut tests)| {
            tests.sort();
            RunnerPlan {
                kind,
                mode,
                program: locate_program(repository, &root, kind),
                prefix_args: prefix_args(kind, mode),
                root,
                tests,
            }
        })
        .collect()
}

pub fn execute(plan: &super::model::ImpactPlan, runner_args: &[String]) -> Result<ExitCode> {
    if plan.runners.is_empty() {
        println!("sensez reflexez: no affected tests");
        return Ok(ExitCode::SUCCESS);
    }
    let mut failed = false;
    for runner in &plan.runners {
        let status = Command::new(&runner.program)
            .args(&runner.prefix_args)
            .args(&runner.tests)
            .args(runner_args)
            .current_dir(&runner.root)
            .status()
            .with_context(|| {
                format!(
                    "starting {} at {} (install it in the project or select another runner)",
                    runner.kind.label(),
                    runner.program.display()
                )
            })?;
        failed |= !status.success() && !is_empty_narrowed_pytest(runner, status.code());
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// pytest exits 5 when it collects nothing, which a narrowed selection may legitimately do.
fn is_empty_narrowed_pytest(runner: &RunnerPlan, code: Option<i32>) -> bool {
    runner.kind == RunnerKind::Pytest && runner.mode == RunMode::Narrowed && code == Some(5)
}

fn prefix_args(kind: RunnerKind, mode: RunMode) -> Vec<String> {
    let args: &[&str] = match (kind, mode) {
        (RunnerKind::Pytest, _) | (RunnerKind::Jest, RunMode::Full) => &[],
        (RunnerKind::Vitest, RunMode::Full) => &["run"],
        (RunnerKind::Vitest, RunMode::Narrowed) => &["run", "--passWithNoTests"],
        (RunnerKind::Jest, RunMode::Narrowed) => &["--runTestsByPath", "--passWithNoTests"],
    };
    args.iter().map(|arg| arg.to_string()).collect()
}

fn locate_program(repository: &Path, runner_root: &Path, kind: RunnerKind) -> PathBuf {
    let name = kind.label();
    let local = executable(runner_root, name);
    if local.is_file() {
        return local;
    }
    let workspace = executable(repository, name);
    if workspace.is_file() {
        return workspace;
    }
    if kind == RunnerKind::Pytest {
        for environment in [".venv", "venv"] {
            let candidate = python_environment(runner_root, environment, name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    PathBuf::from(name)
}

#[cfg(not(windows))]
fn executable(root: &Path, name: &str) -> PathBuf {
    root.join("node_modules/.bin").join(name)
}

#[cfg(windows)]
fn executable(root: &Path, name: &str) -> PathBuf {
    root.join("node_modules/.bin").join(format!("{name}.cmd"))
}

#[cfg(not(windows))]
fn python_environment(root: &Path, environment: &str, name: &str) -> PathBuf {
    root.join(environment).join("bin").join(name)
}

#[cfg(windows)]
fn python_environment(root: &Path, environment: &str, name: &str) -> PathBuf {
    root.join(environment)
        .join("Scripts")
        .join(format!("{name}.exe"))
}

#[cfg(test)]
mod tests;

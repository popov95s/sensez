//! Runner adapters build ordinary commands; test collection/execution stays native.

use super::discovery::TestFile;
use super::model::{RunnerKind, RunnerPlan};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

pub fn plans(
    repository: &Path,
    tests: &[TestFile],
    selected: &HashSet<PathBuf>,
) -> Vec<RunnerPlan> {
    let mut groups: BTreeMap<(RunnerKind, PathBuf), Vec<PathBuf>> = BTreeMap::new();
    for test in tests.iter().filter(|test| selected.contains(&test.file)) {
        groups
            .entry((test.runner, test.runner_root.clone()))
            .or_default()
            .push(test.file.clone());
    }
    groups
        .into_iter()
        .map(|((kind, root), mut tests)| {
            tests.sort();
            RunnerPlan {
                kind,
                program: locate_program(repository, &root, kind),
                prefix_args: prefix_args(kind),
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
        failed |= !status.success();
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn prefix_args(kind: RunnerKind) -> Vec<String> {
    match kind {
        RunnerKind::Pytest => Vec::new(),
        RunnerKind::Vitest => vec!["run".into()],
        RunnerKind::Jest => vec!["--runTestsByPath".into()],
    }
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

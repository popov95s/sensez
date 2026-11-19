use crate::reflexez::discovery::{RunnerRoot, TestFile};
use crate::reflexez::model::{RunMode, RunnerKind, RunnerPlan};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

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

pub(super) fn prefix_args(kind: RunnerKind, mode: RunMode) -> Vec<String> {
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

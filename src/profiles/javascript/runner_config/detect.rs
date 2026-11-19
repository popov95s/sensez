//! Which Node test runners a project directory uses.

use super::evaluate::default_export;
use super::locate::{self, MODULE_EXTENSIONS, SCRIPT_EXTENSIONS};
use crate::profiles::runner_config::RunnerKind;
use serde_json::Value;
use std::path::Path;

const NODE_RUNNERS: [RunnerKind; 2] = [RunnerKind::Vitest, RunnerKind::Jest];
const DEPENDENCY_SECTIONS: [&str; 3] = ["dependencies", "devDependencies", "peerDependencies"];
const COMMAND_WRAPPERS: [&str; 9] = [
    "npx",
    "bunx",
    "pnpm",
    "yarn",
    "exec",
    "dlx",
    "cross-env",
    "node",
    "env",
];

/// Runners a `package.json` script invokes; else the one its dependencies name, Vitest first.
pub fn manifest_runners(manifest: &Value) -> Vec<RunnerKind> {
    let scripts: Vec<&str> = manifest
        .get("scripts")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|scripts| scripts.values().filter_map(Value::as_str))
        .collect();
    let invoked: Vec<_> = NODE_RUNNERS
        .into_iter()
        .filter(|runner| scripts.iter().any(|script| invokes(script, *runner)))
        .collect();
    if !invoked.is_empty() {
        return invoked;
    }
    NODE_RUNNERS
        .into_iter()
        .find(|runner| depends_on(manifest, *runner))
        .into_iter()
        .collect()
}

/// Runner whose config file sits in `root`, for projects without a runner in `package.json`.
pub fn config_runner(root: &Path) -> Option<RunnerKind> {
    let vitest_config = locate::first(root, &["vitest.config"], &SCRIPT_EXTENSIONS);
    let vitest_workspace = locate::first(
        root,
        &["vitest.workspace", "vitest.projects"],
        &MODULE_EXTENSIONS,
    );
    let vite_test = locate::first(root, &["vite.config"], &SCRIPT_EXTENSIONS)
        .is_some_and(|config| default_export(&config).get("test").is_some());
    if vitest_config.is_some() || vitest_workspace.is_some() || vite_test {
        return Some(RunnerKind::Vitest);
    }
    locate::first(root, &["jest.config"], &MODULE_EXTENSIONS).map(|_| RunnerKind::Jest)
}

fn depends_on(manifest: &Value, runner: RunnerKind) -> bool {
    DEPENDENCY_SECTIONS.iter().any(|section| {
        manifest
            .get(section)
            .and_then(|dependencies| dependencies.get(runner.label()))
            .is_some()
    })
}

/// Whether a command in `script` runs `runner`, e.g. `cross-env CI=1 vitest run`.
fn invokes(script: &str, runner: RunnerKind) -> bool {
    script
        .split(['&', '|', ';'])
        .filter_map(|command| {
            command.split_whitespace().find(|word| {
                !word.contains('=') && !word.starts_with('-') && !COMMAND_WRAPPERS.contains(word)
            })
        })
        .any(|program| names_runner(program, runner.label()))
}

fn names_runner(program: &str, name: &str) -> bool {
    let path = Path::new(program);
    path.file_stem().is_some_and(|stem| stem == name)
        || path.components().any(|part| part.as_os_str() == name)
}

#[cfg(test)]
#[path = "detect_tests.rs"]
mod tests;

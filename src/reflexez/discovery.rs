use super::model::{ConfigWarning, RunnerKind};
use super::runner_config::{self, RunnerConfig, SetupFile};
use crate::cli::spec::RunnerChoice;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

mod pytest;

const REFLEXEZ_EXCLUDE: [&str; 5] = [
    "**/node_modules/**",
    "**/.venv/**",
    "**/venv/**",
    "**/target/**",
    "**/.git/**",
];

#[derive(Clone)]
pub struct TestFile {
    pub file: PathBuf,
    pub runner: RunnerKind,
    pub runner_root: PathBuf,
}

/// A directory where one runner is invoked.
#[derive(Clone)]
pub struct RunnerRoot {
    pub kind: RunnerKind,
    pub root: PathBuf,
}

pub struct ProjectFiles {
    pub sources: Vec<PathBuf>,
    pub tests: Vec<TestFile>,
    pub runners: Vec<RunnerRoot>,
    pub setup_files: Vec<SetupFile>,
    pub config_warnings: Vec<ConfigWarning>,
}

struct NodeProject {
    root: PathBuf,
    runner: RunnerKind,
    config: RunnerConfig,
}

fn node_project(root: PathBuf, runner: RunnerKind) -> NodeProject {
    NodeProject {
        config: runner_config::load(&root, runner),
        root,
        runner,
    }
}

pub fn discover(root: &Path, forced: RunnerChoice) -> Result<ProjectFiles> {
    let exclude: Vec<String> = REFLEXEZ_EXCLUDE
        .iter()
        .map(|glob| glob.to_string())
        .collect();
    let discovery = crate::spine::crawler::discover(root, &exclude, &is_project_file)?;
    for issue in &discovery.issues {
        eprintln!("sensez reflexez: discovery: {}", issue.message);
    }
    let files = discovery.files;
    let sources: Vec<_> = files
        .iter()
        .filter(|path| is_supported_source(path))
        .cloned()
        .collect();
    let node_projects = node_projects(root, &files, forced)?;
    let pytest_roots = pytest::test_roots(root)?;
    let mut tests = Vec::new();
    for file in &sources {
        if is_python_test(file)
            && pytest::is_collected(file, &pytest_roots)
            && matches!(forced, RunnerChoice::Auto | RunnerChoice::Pytest)
        {
            tests.push(TestFile {
                file: file.clone(),
                runner: RunnerKind::Pytest,
                runner_root: root.to_path_buf(),
            });
        }
        if let Some(project) =
            nearest_project(file, &node_projects).filter(|project| project.config.is_test(file))
        {
            tests.push(TestFile {
                file: file.clone(),
                runner: project.runner,
                runner_root: project.root.clone(),
            });
        }
    }
    tests.sort_by(|left, right| left.file.cmp(&right.file));
    tests.dedup_by(|left, right| left.file == right.file && left.runner == right.runner);
    let mut runners = runner_roots(&node_projects);
    if tests.iter().any(|test| test.runner == RunnerKind::Pytest) {
        runners.push(RunnerRoot {
            kind: RunnerKind::Pytest,
            root: root.to_path_buf(),
        });
    }
    Ok(ProjectFiles {
        sources,
        tests,
        runners,
        setup_files: node_projects
            .iter()
            .flat_map(|project| project.config.setup_files.iter().cloned())
            .collect(),
        config_warnings: node_projects
            .iter()
            .flat_map(|project| project.config.warnings.iter().cloned())
            .collect(),
    })
}

/// Node runner roots, minus nested roots an enclosing workspace config already runs.
fn runner_roots(projects: &[NodeProject]) -> Vec<RunnerRoot> {
    projects
        .iter()
        .filter(|project| {
            !projects.iter().any(|outer| {
                outer.root != project.root
                    && outer.runner == project.runner
                    && outer.config.project_dirs().any(|dir| dir == project.root)
            })
        })
        .map(|project| RunnerRoot {
            kind: project.runner,
            root: project.root.clone(),
        })
        .collect()
}

fn node_projects(root: &Path, files: &[PathBuf], forced: RunnerChoice) -> Result<Vec<NodeProject>> {
    if matches!(forced, RunnerChoice::Vitest | RunnerChoice::Jest) {
        return Ok(vec![node_project(
            root.to_path_buf(),
            if forced == RunnerChoice::Vitest {
                RunnerKind::Vitest
            } else {
                RunnerKind::Jest
            },
        )]);
    }
    if forced == RunnerChoice::Pytest {
        return Ok(Vec::new());
    }
    let mut projects = Vec::new();
    for manifest in files.iter().filter(|path| path.ends_with("package.json")) {
        let text = std::fs::read_to_string(manifest)
            .with_context(|| format!("reading {}", manifest.display()))?;
        let value: Value = serde_json::from_str(&text)
            .with_context(|| format!("parsing {}", manifest.display()))?;
        let Some(runner) = manifest_runner(&value) else {
            continue;
        };
        if let Some(parent) = manifest.parent() {
            projects.push(node_project(parent.to_path_buf(), runner));
        }
    }
    if projects.is_empty() {
        if has_config(root, "vitest") {
            projects.push(node_project(root.to_path_buf(), RunnerKind::Vitest));
        } else if has_config(root, "jest") {
            projects.push(node_project(root.to_path_buf(), RunnerKind::Jest));
        }
    }
    Ok(projects)
}

fn manifest_runner(value: &Value) -> Option<RunnerKind> {
    let sections = ["dependencies", "devDependencies", "peerDependencies"];
    let has = |name: &str| {
        sections
            .iter()
            .any(|section| value.get(section).and_then(|v| v.get(name)).is_some())
            || value
                .get("scripts")
                .and_then(Value::as_object)
                .is_some_and(|scripts| {
                    scripts
                        .values()
                        .any(|v| v.as_str().is_some_and(|s| s.contains(name)))
                })
    };
    if has("vitest") {
        Some(RunnerKind::Vitest)
    } else if has("jest") {
        Some(RunnerKind::Jest)
    } else {
        None
    }
}

fn nearest_project<'a>(file: &Path, projects: &'a [NodeProject]) -> Option<&'a NodeProject> {
    projects
        .iter()
        .filter(|project| file.starts_with(&project.root))
        .max_by_key(|project| project.root.components().count())
}

fn has_config(root: &Path, runner: &str) -> bool {
    root.join(format!("{runner}.config.js")).is_file()
        || root.join(format!("{runner}.config.mjs")).is_file()
        || root.join(format!("{runner}.config.cjs")).is_file()
        || root.join(format!("{runner}.config.ts")).is_file()
        || root.join(format!("{runner}.config.mts")).is_file()
        || root.join(format!("{runner}.config.cts")).is_file()
}

fn is_supported_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("py" | "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts")
    ) && crate::profiles::registry::should_parse_path(path)
}

fn is_project_file(path: &Path) -> bool {
    is_supported_source(path)
        || path.file_name().and_then(|name| name.to_str()) == Some("package.json")
}

fn is_python_test(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    path.extension().is_some_and(|ext| ext == "py")
        && (name.starts_with("test_") || name.ends_with("_test.py"))
}

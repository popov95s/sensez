use super::model::{ConfigWarning, RunnerKind};
use super::runner_config::{self, RunnerConfig, SetupFile};
use crate::cli::spec::RunnerChoice;
use anyhow::Result;
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
    let (node_projects, manifest_warnings) = node_projects(root, &files, forced);
    let mut pytest_configs = pytest::PytestConfigs::new(root);
    let runs_pytest = matches!(forced, RunnerChoice::Auto | RunnerChoice::Pytest);
    let mut tests = Vec::new();
    for file in &sources {
        let pytest = pytest_configs.for_file(file);
        if runs_pytest && pytest.collects(file) {
            tests.push(TestFile {
                file: file.clone(),
                runner: RunnerKind::Pytest,
                runner_root: pytest.dir.clone(),
            });
        }
        for project in nearest_projects(file, &node_projects) {
            if project.config.is_test(file) {
                tests.push(TestFile {
                    file: file.clone(),
                    runner: project.runner,
                    runner_root: project.root.clone(),
                });
            }
        }
    }
    tests.sort_by(|left, right| left.file.cmp(&right.file));
    tests.dedup_by(|left, right| left.file == right.file && left.runner == right.runner);
    let mut runners = runner_roots(&node_projects, &tests);
    let mut pytest_roots: Vec<_> = tests
        .iter()
        .filter(|test| test.runner == RunnerKind::Pytest)
        .map(|test| test.runner_root.clone())
        .collect();
    pytest_roots.sort();
    pytest_roots.dedup();
    runners.extend(pytest_roots.into_iter().map(|root| RunnerRoot {
        kind: RunnerKind::Pytest,
        root,
    }));
    Ok(ProjectFiles {
        sources,
        tests,
        runners,
        setup_files: node_projects
            .iter()
            .flat_map(|project| project.config.setup_files.iter().cloned())
            .collect(),
        config_warnings: manifest_warnings
            .into_iter()
            .chain(
                node_projects
                    .iter()
                    .flat_map(|project| project.config.warnings.iter().cloned()),
            )
            .collect(),
    })
}

/// Node runner roots with discovered tests, minus nested roots an enclosing workspace config already runs.
fn runner_roots(projects: &[NodeProject], tests: &[TestFile]) -> Vec<RunnerRoot> {
    projects
        .iter()
        .filter(|project| runs_tests(project, tests))
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

/// Whether a discovered test runs from `project`'s root or a project directory it configures.
fn runs_tests(project: &NodeProject, tests: &[TestFile]) -> bool {
    tests.iter().any(|test| {
        test.runner == project.runner
            && (test.runner_root == project.root
                || project
                    .config
                    .project_dirs()
                    .any(|dir| dir == test.runner_root))
    })
}

fn node_projects(
    root: &Path,
    files: &[PathBuf],
    forced: RunnerChoice,
) -> (Vec<NodeProject>, Vec<ConfigWarning>) {
    match forced {
        RunnerChoice::Vitest => {
            return (
                vec![node_project(root.to_path_buf(), RunnerKind::Vitest)],
                Vec::new(),
            )
        }
        RunnerChoice::Jest => {
            return (
                vec![node_project(root.to_path_buf(), RunnerKind::Jest)],
                Vec::new(),
            )
        }
        RunnerChoice::Pytest => return (Vec::new(), Vec::new()),
        RunnerChoice::Auto => {}
    }
    let mut projects = Vec::new();
    let mut warnings = Vec::new();
    for manifest in files.iter().filter(|path| path.ends_with("package.json")) {
        let value = match read_manifest(manifest) {
            Ok(value) => value,
            Err(reason) => {
                warnings.push(ConfigWarning {
                    file: manifest.clone(),
                    key: "package.json".into(),
                    reason,
                    fallback: "no Node runner for this directory".into(),
                });
                continue;
            }
        };
        if let Some(parent) = manifest.parent() {
            for runner in runner_config::manifest_runners(&value) {
                projects.push(node_project(parent.to_path_buf(), runner));
            }
        }
    }
    if projects.is_empty() {
        projects.extend(
            runner_config::config_runner(root)
                .map(|runner| node_project(root.to_path_buf(), runner)),
        );
    }
    (projects, warnings)
}

fn read_manifest(manifest: &Path) -> std::result::Result<Value, String> {
    let text = std::fs::read_to_string(manifest).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

/// Projects rooted in the deepest directory above `file`; one per runner.
fn nearest_projects<'a>(file: &Path, projects: &'a [NodeProject]) -> Vec<&'a NodeProject> {
    let enclosing: Vec<_> = projects
        .iter()
        .filter(|project| file.starts_with(&project.root))
        .collect();
    let deepest = enclosing
        .iter()
        .map(|project| project.root.components().count())
        .max();
    enclosing
        .into_iter()
        .filter(|project| Some(project.root.components().count()) == deepest)
        .collect()
}

fn is_supported_source(path: &Path) -> bool {
    super::changes::is_source(path) && crate::profiles::registry::should_parse_path(path)
}

fn is_project_file(path: &Path) -> bool {
    is_supported_source(path)
        || path.file_name().and_then(|name| name.to_str()) == Some("package.json")
}

use super::defaults::{owned, VITEST_EXCLUDE, VITEST_INCLUDE};
use super::evaluate::default_export;
use super::key::{self, Key};
use super::locate::{self, MAX_PROJECT_DEPTH, MODULE_EXTENSIONS, SCRIPT_EXTENSIONS};
use super::project::{Exclude, Include, PathBase, TestProject};
use super::projects::{self, Entry};
use super::value::{merge, Object, Value};
use super::{glob_set, RunnerConfig, HEURISTIC_FALLBACK};
use globset::GlobSet;
use std::path::Path;

const CONFIG_STEMS: [&str; 2] = ["vitest.config", "vite.config"];
const WORKSPACE_STEMS: [&str; 2] = ["vitest.workspace", "vitest.projects"];

pub fn read(root: &Path) -> RunnerConfig {
    let mut out = RunnerConfig::default();
    let workspace = locate::first(root, &WORKSPACE_STEMS, &MODULE_EXTENSIONS);
    match locate::first(root, &CONFIG_STEMS, &SCRIPT_EXTENSIONS) {
        Some(file) => {
            let config = default_export(&file);
            read_config(&mut out, &config, &file, root, workspace.as_deref(), 0);
        }
        None => match workspace {
            Some(file) => read_workspace(&mut out, &file, &Value::Other, 0),
            None => {
                let project = project(&mut out, &empty(), root, root);
                out.projects.push(project);
            }
        },
    }
    out
}

fn read_config(
    out: &mut RunnerConfig,
    config: &Value,
    file: &Path,
    parent: &Path,
    workspace: Option<&Path>,
    depth: usize,
) {
    let test = match config.get("test") {
        None | Some(Value::Other) => empty(),
        Some(test) => test,
    };
    if let Value::Opaque(reason) = test {
        out.warn(file, "test", reason, HEURISTIC_FALLBACK);
        out.projects.push(heuristic_project(parent));
        return;
    }
    let root_source = match key::read(&test, "root") {
        Key::Absent => config,
        _ => &test,
    };
    let root = out.read_dir(root_source, file, "root", parent);
    out.read_setup(&test, file, "setupFiles", &root);
    out.read_setup(&test, file, "globalSetup", &root);
    let projects = test
        .get("projects")
        .or_else(|| test.get("workspace"))
        .filter(|value| *value != Value::Other);
    match (projects, workspace) {
        (Some(projects), _) => {
            let before = out.projects.len();
            read_projects(out, &projects, file, &root, config, depth);
            if out.projects.len() == before {
                let project = project(out, &test, file, &root);
                out.projects.push(project);
            }
        }
        (None, Some(workspace)) => read_workspace(out, workspace, config, depth),
        (None, None) => {
            let project = project(out, &test, file, &root);
            out.projects.push(project);
        }
    }
}

fn read_workspace(out: &mut RunnerConfig, file: &Path, parent_config: &Value, depth: usize) {
    let base = file.parent().unwrap_or(Path::new("."));
    let value = default_export(file);
    read_projects(out, &value, file, base, parent_config, depth);
}

fn read_projects(
    out: &mut RunnerConfig,
    value: &Value,
    file: &Path,
    root: &Path,
    parent_config: &Value,
    depth: usize,
) {
    if depth >= MAX_PROJECT_DEPTH {
        out.warn(
            file,
            "projects",
            "projects nest too deeply".into(),
            HEURISTIC_FALLBACK,
        );
        out.projects.push(heuristic_project(root));
        return;
    }
    let (entries, unreadable) = projects::entries(value, root);
    for reason in unreadable {
        out.warn(file, "projects", reason, HEURISTIC_FALLBACK);
        out.projects.push(heuristic_project(root));
    }
    for entry in entries {
        match entry {
            Entry::Inline(project) => {
                let inherited = inherit(project, parent_config, root);
                read_config(out, &inherited, file, root, None, depth + 1);
            }
            Entry::Path(path) if path.is_file() => {
                let config = default_export(&path);
                let dir = path.parent().unwrap_or(root);
                read_config(out, &config, &path, dir, None, depth + 1);
            }
            Entry::Path(dir) => match locate::first(&dir, &CONFIG_STEMS, &SCRIPT_EXTENSIONS) {
                Some(path) => {
                    let config = default_export(&path);
                    read_config(out, &config, &path, &dir, None, depth + 1);
                }
                None => {
                    let project = project(out, &empty(), file, &dir);
                    out.projects.push(project);
                }
            },
        }
    }
}

/// Apply a project's `extends` (`true` inherits the root `test` options, a string names a config).
fn inherit(project: Value, parent_config: &Value, root: &Path) -> Value {
    match project.get("extends") {
        Some(Value::Bool(true)) => {
            let parent_test = parent_config
                .get("test")
                .unwrap_or_else(empty)
                .without("projects")
                .without("workspace");
            let parent = Value::Object(Object {
                entries: vec![("test".into(), parent_test)],
                unknown: None,
            });
            merge(parent, project)
        }
        Some(Value::Str(path)) => merge(default_export(&root.join(path)), project),
        _ => project,
    }
}

fn project(out: &mut RunnerConfig, test: &Value, file: &Path, root: &Path) -> TestProject {
    let dir = out.read_dir(test, file, "dir", root);
    let mut negated = Vec::new();
    let include = match key::read(test, "include") {
        Key::Absent => Ok(owned(&VITEST_INCLUDE)),
        Key::Strings(patterns) => {
            let (negations, positives): (Vec<_>, Vec<_>) = patterns
                .into_iter()
                .partition(|pattern| pattern.starts_with('!'));
            negated = negations
                .iter()
                .map(|pattern| pattern[1..].to_string())
                .collect();
            Ok(positives)
        }
        Key::Unreadable(reason) => Err(reason),
    }
    .and_then(|patterns| glob_set(&patterns));
    let include = include.map_or_else(
        |reason| {
            out.warn(file, "include", reason, HEURISTIC_FALLBACK);
            Include::NameHeuristic
        },
        Include::Globs,
    );
    let exclude = match key::read(test, "exclude") {
        Key::Absent => Ok(owned(&VITEST_EXCLUDE)),
        Key::Strings(patterns) => Ok(patterns),
        Key::Unreadable(reason) => Err(reason),
    }
    .and_then(|mut patterns| {
        patterns.extend(negated);
        glob_set(&patterns)
    });
    let exclude = exclude.unwrap_or_else(|reason| {
        out.warn(file, "exclude", reason, "Vitest default exclude");
        default_exclude()
    });
    TestProject {
        roots: vec![dir.clone()],
        dir,
        base: PathBase::Relative,
        include,
        exclude: Exclude::Globs(exclude),
    }
}

fn heuristic_project(root: &Path) -> TestProject {
    TestProject {
        dir: root.to_path_buf(),
        base: PathBase::Relative,
        roots: vec![root.to_path_buf()],
        include: Include::NameHeuristic,
        exclude: Exclude::Globs(default_exclude()),
    }
}

fn default_exclude() -> GlobSet {
    glob_set(&owned(&VITEST_EXCLUDE)).unwrap_or_else(|_| GlobSet::empty())
}

fn empty() -> Value {
    Value::Object(Object::default())
}

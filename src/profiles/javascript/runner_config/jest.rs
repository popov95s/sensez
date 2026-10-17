use super::defaults::{owned, JEST_IGNORE, JEST_TEST_MATCH};
use super::evaluate::default_export;
use super::key::{self, Key};
use super::locate::{self, MAX_PROJECT_DEPTH, MODULE_EXTENSIONS};
use super::project::{Exclude, Include, PathBase, TestProject};
use super::projects::{self, Entry};
use super::value::{Object, Value};
use super::{glob_set, RunnerConfig, HEURISTIC_FALLBACK};
use crate::profiles::javascript::roots::normalize as normalize_path;
use regex::Regex;
use std::path::{Path, PathBuf};

pub fn read(root: &Path) -> RunnerConfig {
    let mut out = RunnerConfig::default();
    let (config, file) = locate_config(root);
    read_config(&mut out, &config, &file, root, 0);
    out
}

/// The jest config in `dir`: `jest.config.*`, else the `package.json` `"jest"` key.
fn locate_config(dir: &Path) -> (Value, PathBuf) {
    if let Some(file) = locate::first(dir, &["jest.config"], &MODULE_EXTENSIONS) {
        return (default_export(&file), file);
    }
    let manifest = dir.join("package.json");
    let config = std::fs::read_to_string(&manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|json| json.get("jest").map(Value::from_json))
        .unwrap_or_else(|| Value::Object(Object::default()));
    (config, manifest)
}

fn read_config(out: &mut RunnerConfig, config: &Value, file: &Path, dir: &Path, depth: usize) {
    if let Value::Opaque(reason) = config {
        out.warn(file, "config", reason.clone(), HEURISTIC_FALLBACK);
        out.projects.push(heuristic_project(dir));
        return;
    }
    let root_dir = out.read_dir(config, file, "rootDir", dir);
    for key in ["setupFiles", "setupFilesAfterEnv", "globalSetup"] {
        out.read_setup(config, file, key, &root_dir);
    }
    match config
        .get("projects")
        .filter(|value| *value != Value::Other)
    {
        Some(projects) if depth < MAX_PROJECT_DEPTH => {
            let before = out.projects.len();
            read_projects(out, &projects, file, &root_dir, depth);
            if out.projects.len() == before {
                let project = project(out, config, file, &root_dir);
                out.projects.push(project);
            }
        }
        Some(_) => {
            out.warn(
                file,
                "projects",
                "projects nest too deeply".into(),
                HEURISTIC_FALLBACK,
            );
            out.projects.push(heuristic_project(&root_dir));
        }
        None => {
            let project = project(out, config, file, &root_dir);
            out.projects.push(project);
        }
    }
}

fn read_projects(
    out: &mut RunnerConfig,
    value: &Value,
    file: &Path,
    root_dir: &Path,
    depth: usize,
) {
    let with_root = with_root_dir(value, root_dir);
    let (entries, unreadable) = projects::entries(&with_root, root_dir);
    for reason in unreadable {
        out.warn(file, "projects", reason, HEURISTIC_FALLBACK);
        out.projects.push(heuristic_project(root_dir));
    }
    for entry in entries {
        match entry {
            Entry::Inline(project) => read_config(out, &project, file, root_dir, depth + 1),
            Entry::Path(path) if path.is_file() => {
                let dir = path.parent().unwrap_or(root_dir).to_path_buf();
                read_config(out, &default_export(&path), &path, &dir, depth + 1);
            }
            Entry::Path(dir) => {
                let (config, file) = locate_config(&dir);
                read_config(out, &config, &file, &dir, depth + 1);
            }
        }
    }
}

fn project(out: &mut RunnerConfig, config: &Value, file: &Path, root_dir: &Path) -> TestProject {
    let roots = match key::read(config, "roots") {
        Key::Strings(roots) => roots
            .iter()
            .map(|root| normalize_path(&root_dir.join(root.replace("<rootDir>", "."))))
            .collect(),
        Key::Unreadable(reason) => {
            out.warn(file, "roots", reason, "rootDir");
            vec![root_dir.to_path_buf()]
        }
        Key::Absent => vec![root_dir.to_path_buf()],
    };
    let root_text = root_dir.to_string_lossy().replace('\\', "/");
    let expand = |patterns: Vec<String>| -> Vec<String> {
        patterns
            .iter()
            .map(|pattern| pattern.replace("<rootDir>", &root_text))
            .collect()
    };
    let include = match (
        key::read(config, "testRegex"),
        key::read(config, "testMatch"),
    ) {
        (Key::Strings(patterns), _) => regexes(&expand(patterns))
            .map(Include::Regexes)
            .map_err(|reason| ("testRegex", reason)),
        (Key::Unreadable(reason), _) => Err(("testRegex", reason)),
        (Key::Absent, Key::Strings(patterns)) => glob_set(&expand(patterns))
            .map(Include::Globs)
            .map_err(|reason| ("testMatch", reason)),
        (Key::Absent, Key::Unreadable(reason)) => Err(("testMatch", reason)),
        (Key::Absent, Key::Absent) => glob_set(&owned(&JEST_TEST_MATCH))
            .map(Include::Globs)
            .map_err(|reason| ("testMatch", reason)),
    };
    let include = include.unwrap_or_else(|(key, reason)| {
        out.warn(file, key, reason, HEURISTIC_FALLBACK);
        Include::NameHeuristic
    });
    let ignore = match key::read(config, "testPathIgnorePatterns") {
        Key::Strings(patterns) => regexes(&expand(patterns)),
        Key::Unreadable(reason) => Err(reason),
        Key::Absent => regexes(&owned(&JEST_IGNORE)),
    };
    let ignore = ignore.unwrap_or_else(|reason| {
        out.warn(
            file,
            "testPathIgnorePatterns",
            reason,
            "Jest default testPathIgnorePatterns",
        );
        default_ignore()
    });
    TestProject {
        dir: root_dir.to_path_buf(),
        base: PathBase::Absolute,
        roots,
        include,
        exclude: Exclude::Regexes(ignore),
    }
}

/// Inline project objects inherit the parent `rootDir` unless they set their own.
fn with_root_dir(value: &Value, root_dir: &Path) -> Value {
    let Value::Array(items) = value else {
        return value.clone();
    };
    let root = root_dir.to_string_lossy().into_owned();
    Value::Array(
        items
            .iter()
            .map(|item| match item {
                Value::Str(path) => Value::Str(path.replace("<rootDir>", &root)),
                Value::Object(object) if item.get("rootDir").is_none() => {
                    let mut object = object.clone();
                    object
                        .entries
                        .push(("rootDir".into(), Value::Str(root.clone())));
                    Value::Object(object)
                }
                other => other.clone(),
            })
            .collect(),
    )
}

fn regexes(patterns: &[String]) -> Result<Vec<Regex>, String> {
    patterns
        .iter()
        .map(|pattern| Regex::new(pattern))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn heuristic_project(dir: &Path) -> TestProject {
    TestProject {
        dir: dir.to_path_buf(),
        base: PathBase::Absolute,
        roots: vec![dir.to_path_buf()],
        include: Include::NameHeuristic,
        exclude: Exclude::Regexes(default_ignore()),
    }
}

fn default_ignore() -> Vec<Regex> {
    JEST_IGNORE
        .iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect()
}

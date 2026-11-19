//! Declared dependencies from `pyproject.toml` and `requirements*.txt`.

use crate::profiles::dependencies::{manifest_dirs, DeclaredPackages, DependencyScope};
use std::path::Path;
use toml::Value;

const PYPROJECT: &str = "pyproject.toml";
const DEVELOPMENT_GROUPS: [&str; 7] = ["dev", "test", "tests", "testing", "lint", "docs", "typing"];

/// Packages every Python manifest from `dir` up to the repository root declares.
pub fn declared_packages(dir: &Path) -> DeclaredPackages {
    let mut declared = DeclaredPackages::default();
    for dir in manifest_dirs(dir) {
        if let Some(pyproject) = read_pyproject(&dir) {
            declare_pyproject(&pyproject, &mut declared);
        }
        for (name, text) in requirement_files(&dir) {
            let scope = group_scope(&name);
            for package in text.lines().filter_map(requirement_name) {
                declared.declare(scope, package, &name);
            }
        }
    }
    declared
}

fn read_pyproject(dir: &Path) -> Option<Value> {
    std::fs::read_to_string(dir.join(PYPROJECT))
        .ok()?
        .parse()
        .ok()
}

fn declare_pyproject(pyproject: &Value, declared: &mut DeclaredPackages) {
    let mut declare = |scope, specs: Vec<String>| {
        for package in specs.iter().filter_map(|spec| requirement_name(spec)) {
            declared.declare(scope, package, PYPROJECT);
        }
    };
    let project = pyproject.get("project");
    declare(
        DependencyScope::Production,
        strings(project.and_then(|p| p.get("dependencies"))),
    );
    for (group, specs) in tables(project.and_then(|p| p.get("optional-dependencies"))) {
        declare(group_scope(group), strings(Some(specs)));
    }
    for (_, specs) in tables(pyproject.get("dependency-groups")) {
        declare(DependencyScope::Development, strings(Some(specs)));
    }
    let poetry = pyproject.get("tool").and_then(|tool| tool.get("poetry"));
    declare(
        DependencyScope::Production,
        keys(poetry.and_then(|p| p.get("dependencies"))),
    );
    declare(
        DependencyScope::Development,
        keys(poetry.and_then(|p| p.get("dev-dependencies"))),
    );
    for (group, table) in tables(poetry.and_then(|p| p.get("group"))) {
        declare(group_scope(group), keys(table.get("dependencies")));
    }
}

fn strings(value: Option<&Value>) -> Vec<String> {
    let items = value.and_then(Value::as_array).into_iter().flatten();
    items
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn keys(value: Option<&Value>) -> Vec<String> {
    let table = value.and_then(Value::as_table).into_iter().flatten();
    table.map(|(key, _)| key.clone()).collect()
}

fn tables(value: Option<&Value>) -> impl Iterator<Item = (&str, &Value)> {
    let table = value.and_then(Value::as_table).into_iter().flatten();
    table.map(|(key, value)| (key.as_str(), value))
}

fn requirement_files(dir: &Path) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("requirements") && name.ends_with(".txt"))
        .filter_map(|name| Some((name.clone(), std::fs::read_to_string(dir.join(&name)).ok()?)))
        .collect()
}

/// Development when the group or file name mentions a development-only group.
fn group_scope(name: &str) -> DependencyScope {
    let lowered = name.to_ascii_lowercase();
    match DEVELOPMENT_GROUPS
        .iter()
        .any(|group| lowered.contains(group))
    {
        true => DependencyScope::Development,
        false => DependencyScope::Production,
    }
}

/// Normalized distribution name of one requirement line (`Foo_Bar[x]>=1` → `foo-bar`).
fn requirement_name(line: &str) -> Option<String> {
    let spec = line.split('#').next()?.trim();
    let name: String = spec
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    let is_package = spec.starts_with(|c: char| c.is_ascii_alphanumeric()) && name != "python";
    is_package.then(|| name.to_ascii_lowercase().replace(['_', '.'], "-"))
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;

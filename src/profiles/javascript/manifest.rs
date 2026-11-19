//! `package.json` reading: entry targets as dead-code roots, dependencies for utility lookups.

use super::{roots, workflows};
use crate::profiles::dependencies::{manifest_dirs, DeclaredPackages, DependencyScope};
use crate::profiles::EntryCandidate;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const ENTRY_FIELDS: [&str; 4] = ["main", "module", "bin", "exports"];
const BUILD_DIRS: [&str; 4] = ["dist", "lib", "build", "out"];
const MANIFEST: &str = "package.json";
const DEPENDENCY_SECTIONS: [(&str, DependencyScope); 4] = [
    ("dependencies", DependencyScope::Production),
    ("peerDependencies", DependencyScope::Production),
    ("optionalDependencies", DependencyScope::Production),
    ("devDependencies", DependencyScope::Development),
];

/// Files named by their package's manifest (directly or through a `src/` twin of a build path) or run as programs.
pub fn entry_files(files: &[EntryCandidate]) -> Vec<PathBuf> {
    let mut keys_by_root: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    let mut entries = Vec::new();
    for file in files {
        let root = roots::root_for(file.path);
        let key = roots::module_name(file.path, &root);
        let keys = keys_by_root
            .entry(root)
            .or_insert_with_key(|root| manifest_keys(root));
        if keys.contains(&key) || file.runs_as_program {
            entries.push(file.path.to_path_buf());
        }
    }
    entries
}

/// Packages every `package.json` from `dir` up to the repository root declares.
pub fn declared_packages(dir: &Path) -> DeclaredPackages {
    let mut declared = DeclaredPackages::default();
    for manifest in manifest_dirs(dir)
        .iter()
        .filter_map(|dir| read_manifest(dir))
    {
        for (section, scope) in DEPENDENCY_SECTIONS {
            let packages = manifest.get(section).and_then(Value::as_object);
            for package in packages.into_iter().flat_map(|packages| packages.keys()) {
                declared.declare(scope, package.clone(), MANIFEST);
            }
        }
    }
    declared
}

fn read_manifest(dir: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(dir.join(MANIFEST)).ok()?;
    serde_json::from_str(&text).ok()
}

fn manifest_keys(root: &Path) -> HashSet<String> {
    let Some(manifest) = read_manifest(root) else {
        return HashSet::new();
    };
    let mut targets = Vec::new();
    for field in ENTRY_FIELDS {
        if let Some(value) = manifest.get(field) {
            collect_strings(value, &mut targets);
        }
    }
    let mut commands = Vec::new();
    if let Some(scripts) = manifest.get("scripts") {
        collect_strings(scripts, &mut commands);
    }
    let workflow_commands = workflows::run_commands(root);
    targets.extend(
        commands
            .iter()
            .copied()
            .chain(workflow_commands.iter().map(String::as_str))
            .flat_map(script_files),
    );
    targets
        .iter()
        .filter(|target| !target.contains('*'))
        .flat_map(|target| source_keys(target))
        .collect()
}

fn collect_strings<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::String(text) => out.push(text),
        Value::Array(items) => items.iter().for_each(|item| collect_strings(item, out)),
        Value::Object(map) => map.values().for_each(|item| collect_strings(item, out)),
        _ => {}
    }
}

/// Source files a `scripts` command runs, e.g. `node ./tools/sync.mjs --dry`.
fn script_files(command: &str) -> impl Iterator<Item = &str> {
    command
        .split_whitespace()
        .map(|token| token.trim_matches(|c| c == '"' || c == '\''))
        .filter(|token| !token.starts_with('-') && roots::has_source_extension(Path::new(token)))
}

fn source_keys(target: &str) -> Vec<String> {
    let parts: Vec<String> = roots::normalize(Path::new(target))
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let mut keys = vec![roots::key_from_parts(parts.clone())];
    if parts
        .first()
        .is_some_and(|first| BUILD_DIRS.contains(&first.as_str()))
    {
        let mut twin = parts;
        twin[0] = "src".to_string();
        keys.push(roots::key_from_parts(twin));
    }
    keys
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;

//! `package.json` entry targets (`main`, `module`, `bin`, `exports`) as dead-code roots.

use super::roots;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const ENTRY_FIELDS: [&str; 4] = ["main", "module", "bin", "exports"];
const BUILD_DIRS: [&str; 4] = ["dist", "lib", "build", "out"];

/// Files named by their package's manifest, directly or through a `src/` twin of a build path.
pub fn entry_files(files: &[&Path]) -> Vec<PathBuf> {
    let mut keys_by_root: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    let mut entries = Vec::new();
    for file in files {
        let root = roots::root_for(file);
        let key = roots::module_name(file, &root);
        let keys = keys_by_root
            .entry(root)
            .or_insert_with_key(|root| manifest_keys(root));
        if keys.contains(&key) {
            entries.push(file.to_path_buf());
        }
    }
    entries
}

fn manifest_keys(root: &Path) -> HashSet<String> {
    let Some(manifest) = std::fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    else {
        return HashSet::new();
    };
    let mut targets = Vec::new();
    for field in ENTRY_FIELDS {
        if let Some(value) = manifest.get(field) {
            collect_strings(value, &mut targets);
        }
    }
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
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn manifest_targets_map_to_source_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("package.json"),
            r#"{"main": "./dist/index.js", "bin": {"tool": "bin/cli.mjs"}, "exports": {".": {"import": "./lib/esm.js"}, "./*": "./dist/*.js"}}"#,
        )
        .unwrap();
        let paths: Vec<PathBuf> = ["src/index.ts", "bin/cli.mjs", "src/esm.ts", "src/other.ts"]
            .iter()
            .map(|name| root.join(name))
            .collect();
        for path in &paths {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        let files: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();

        let entries = entry_files(&files);

        assert_eq!(entries, paths[..3].to_vec());
    }
}

use super::*;
use std::fs;

#[test]
fn manifest_targets_map_to_source_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("package.json"),
        r#"{"main": "./dist/index.js", "bin": {"tool": "bin/cli.mjs"}, "exports": {".": {"import": "./lib/esm.js"}, "./*": "./dist/*.js"}, "scripts": {"sync": "node ./tools/sync.mjs --dry", "lint": "eslint src"}}"#,
    )
    .unwrap();
    let paths: Vec<PathBuf> = [
        "src/index.ts",
        "bin/cli.mjs",
        "src/esm.ts",
        "tools/sync.mjs",
        "src/other.ts",
    ]
    .iter()
    .map(|name| root.join(name))
    .collect();
    for path in &paths {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }
    let files: Vec<EntryCandidate> = paths
        .iter()
        .map(|path| EntryCandidate {
            path,
            runs_as_program: false,
        })
        .collect();

    let entries = entry_files(&files);

    assert_eq!(entries, paths[..4].to_vec());
}

#[test]
fn workflow_run_commands_are_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("package.json"), "{}").unwrap();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::write(
        root.join(".github/workflows/report.yml"),
        "jobs:\n  report:\n    steps:\n      - run: node tools/report.mjs\n",
    )
    .unwrap();
    let paths: Vec<PathBuf> = ["tools/report.mjs", "tools/other.mjs"]
        .iter()
        .map(|name| root.join(name))
        .collect();
    let files: Vec<EntryCandidate> = paths
        .iter()
        .map(|path| EntryCandidate {
            path,
            runs_as_program: false,
        })
        .collect();

    assert_eq!(entry_files(&files), paths[..1].to_vec());
}

#[test]
fn dev_only_packages_are_declared_for_development() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join(MANIFEST),
        r#"{"dependencies": {"es-toolkit": "1"}, "devDependencies": {"remeda": "2"}}"#,
    )
    .unwrap();

    let declared = declared_packages(tmp.path());

    assert!(declared
        .origin("es-toolkit", DependencyScope::Production)
        .is_some());
    assert!(declared
        .origin("remeda", DependencyScope::Production)
        .is_none());
    assert!(declared
        .origin("remeda", DependencyScope::Development)
        .is_some());
}

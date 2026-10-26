use sensez::analyze_path;
use std::fs;
use std::path::{Path, PathBuf};

const SMELL_EXAMPLES: &str = "docs/examples/smells";

/// Kinds whose examples only fire under the tuned thresholds in `docs/examples/sensez.toml`.
const NEEDS_DOCS_THRESHOLDS: &[&str] = &[
    "data_clump",
    "deep_nesting",
    "god_module",
    "heavy_nested_function",
    "high_cognitive_complexity",
    "high_complexity",
    "implicit_schema",
    "large_class",
    "long_function",
    "long_parameter_list",
    "message_chain",
];
const SUFFIXES: [&str; 2] = ["py", "ts"];

fn stage(folder: &Path, suffix: &str, root: &Path) {
    let box_dir = root.join(folder.file_name().unwrap());
    for path in files(folder) {
        let rel = path.strip_prefix(folder).unwrap();
        let first = rel
            .components()
            .next()
            .unwrap()
            .as_os_str()
            .to_string_lossy();
        if path.extension().is_none_or(|ext| ext != suffix) || first.starts_with("fixed") {
            continue;
        }
        let target = box_dir.join(rel);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(&path, target).unwrap();
    }
}

fn files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .flat_map(|path| {
            if path.is_dir() {
                files(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

fn fires_on_defaults(folder: &Path, kind: &str, suffix: &str) -> bool {
    let root = tempfile::tempdir().unwrap();
    stage(folder, suffix, root.path());
    fs::write(
        root.path().join("sensez.toml"),
        format!("[smells.rules.{kind}]\nenabled = true\n"),
    )
    .unwrap();
    let (report, _) = analyze_path(root.path(), None).unwrap();
    report
        .smells
        .iter()
        .any(|finding| finding.kind.as_str() == kind)
}

#[test]
fn every_smell_example_fires_on_default_thresholds() {
    let mut folders: Vec<PathBuf> = fs::read_dir(SMELL_EXAMPLES)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    let mut silent = Vec::new();
    for folder in &folders {
        let kind = folder.file_name().unwrap().to_string_lossy().to_string();
        for suffix in SUFFIXES {
            if !fires_on_defaults(folder, &kind, suffix) {
                silent.push((kind.clone(), suffix));
            }
        }
    }
    let expected: Vec<(String, &str)> = NEEDS_DOCS_THRESHOLDS
        .iter()
        .flat_map(|kind| SUFFIXES.map(|suffix| (kind.to_string(), suffix)))
        .collect();
    assert_eq!(silent, expected);
}

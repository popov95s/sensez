use super::*;
use crate::spine::{graph, parser};
use std::fs;

fn project() -> (tempfile::TempDir, CodebaseGraph, Vec<PathBuf>) {
    let root = tempfile::tempdir().unwrap();
    for (name, source) in [
        ("a.py", "from b import value\n"),
        ("b.py", "from c import value\n"),
        ("c.py", "value = 1\n"),
        ("d.py", "from b import value\n"),
        ("unrelated.py", "value = 2\n"),
    ] {
        fs::write(root.path().join(name), source).unwrap();
    }
    let paths: Vec<_> = ["a.py", "b.py", "c.py", "d.py", "unrelated.py"]
        .into_iter()
        .map(|name| root.path().join(name))
        .collect();
    let parsed = parser::parse_files(&paths);
    assert!(parsed.issues.is_empty());
    let graph = graph::build(&parsed.files, &[]);
    (root, graph, paths)
}

#[test]
fn finds_transitive_callers_and_callees() {
    let (_root, graph, paths) = project();
    let affected = affected_files(&graph, &[paths[1].clone()], ImpactOptions::default());

    assert_eq!(affected.dependencies.files[&paths[1]], 0);
    assert_eq!(affected.dependencies.files[&paths[2]], 1);
    assert_eq!(affected.dependents.files[&paths[1]], 0);
    assert_eq!(affected.dependents.files[&paths[0]], 1);
    assert_eq!(affected.dependents.files[&paths[3]], 1);
    assert!(!affected.all_files().contains_key(&paths[4]));
}

#[test]
fn reports_changed_paths_missing_from_the_graph() {
    let (root, graph, _) = project();
    let missing = root.path().join("removed.py");
    let impact = affected_files(
        &graph,
        std::slice::from_ref(&missing),
        ImpactOptions::default(),
    );

    assert_eq!(impact.unmapped(), BTreeSet::from([missing]));
    assert!(impact.all_files().is_empty());
}

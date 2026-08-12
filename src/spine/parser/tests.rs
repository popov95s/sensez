use super::*;
use crate::profiles::registry;

#[test]
fn pathological_nesting_is_rejected_not_crashed() {
    let profile = registry::parse_for_path(Path::new("x.py")).unwrap();
    let src = format!("x = {}1{}", "(".repeat(100_000), ")".repeat(100_000));
    assert!(parse_source(src.as_bytes(), 0, "x", profile).is_err());
    assert!(parse_source(b"def f():\n    return (1 + 2)\n", 0, "x", profile).is_ok());
}

#[test]
fn depth_gate_boundary_is_exact() {
    let profile = registry::parse_for_path(Path::new("x.py")).unwrap();
    let src_with = |parens: usize| format!("x = {}1{}", "(".repeat(parens), ")".repeat(parens));
    let depth_of = |parens: usize| {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&profile.ts_language()).unwrap();
        let tree = parser.parse(src_with(parens).as_bytes(), None).unwrap();
        tree_depth(tree.root_node(), usize::MAX)
    };
    let at_limit = MAX_TREE_DEPTH - (depth_of(10) - 10);

    assert_eq!(depth_of(at_limit), MAX_TREE_DEPTH);
    assert!(parse_source(src_with(at_limit).as_bytes(), 0, "x", profile).is_ok());
    assert!(parse_source(src_with(at_limit + 1).as_bytes(), 0, "x", profile).is_err());
}

#[test]
fn loaded_sources_parse_without_rereading_the_filesystem() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("loaded.py");
    std::fs::write(&path, "def loaded():\n    return 1\n").unwrap();
    let project = crate::spine::cache::load_project(std::slice::from_ref(&path), 1).unwrap();
    std::fs::remove_file(&path).unwrap();

    let parsed = parse_sources(&project.sources);

    assert!(parsed.issues.is_empty());
    assert_eq!(parsed.files.len(), 1);
    assert_eq!(parsed.files[0].path, path);
}

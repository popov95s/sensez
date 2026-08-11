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
fn cached_spans_are_rebound_to_the_current_file_id() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("cached.py");
    std::fs::write(&path, "def f():\n    return 1\n").unwrap();
    let cache = crate::spine::cache::ParseCache::new(tmp.path());
    let mut parser = tree_sitter::Parser::new();

    let first = parse_file_with_cache(&path, 3, Some(&cache), &mut parser).unwrap();
    assert!(first
        .walked
        .syntax
        .spans
        .iter()
        .all(|span| span.file_id == 3));
    let second = parse_file_with_cache(&path, 7, Some(&cache), &mut parser).unwrap();
    assert!(second
        .walked
        .syntax
        .spans
        .iter()
        .all(|span| span.file_id == 7));
}

use super::*;
use crate::spine::parser::parse_file;
use std::fs;

#[test]
fn detects_two_module_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("a.py"),
        "from b import beta\n\ndef alpha():\n    return beta\n",
    )
    .unwrap();
    fs::write(
        dir.join("b.py"),
        "from a import alpha\n\ndef beta():\n    return alpha\n",
    )
    .unwrap();

    let files: Vec<_> = ["a.py", "b.py"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    let cycles = detect(&cg, &[]);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].modules.len(), 2);
    // Each module contributes a clickable next-hop edge (file + import line).
    assert_eq!(cycles[0].edges.len(), 2);
    assert!(cycles[0]
        .edges
        .iter()
        .all(|e| e.line == 1 && (e.file.ends_with("a.py") || e.file.ends_with("b.py"))));
}

/// Inline (function-local) imports are how Python *breaks* cycles, so a
/// mutual dependency that exists only via inline imports is NOT a cycle.
#[test]
fn inline_imports_do_not_form_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("a.py"),
        "def alpha():\n    from b import beta\n    return beta\n",
    )
    .unwrap();
    fs::write(
        dir.join("b.py"),
        "def beta():\n    from a import alpha\n    return alpha\n",
    )
    .unwrap();

    let files: Vec<_> = ["a.py", "b.py"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    assert!(
        detect(&cg, &[]).is_empty(),
        "inline-only mutual imports must not be a load-time cycle"
    );
}

#[test]
fn type_checking_imports_form_info_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("a.py"),
        "import typing as t\n\nif t.TYPE_CHECKING:\n    from b import B\n\nclass A:\n    pass\n",
    )
    .unwrap();
    fs::write(dir.join("b.py"), "from a import A\n\nclass B:\n    pass\n").unwrap();

    let files: Vec<_> = ["a.py", "b.py"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    let cycles = detect(&cg, &[]);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].action, ActionLevel::Info);
    assert!(cycles[0]
        .hint
        .as_deref()
        .is_some_and(|hint| hint.contains("type coupling")));
    assert!(cycles[0]
        .edges
        .iter()
        .any(|edge| { edge.from_module == "a" && edge.to_module == "b" && edge.line == 4 }));
}

#[test]
fn excluded_test_fixture_cycles_are_silent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("tests/input/cycle")).unwrap();
    fs::write(
        dir.join("tests/input/cycle/a.py"),
        "from tests.input.cycle.b import beta\n",
    )
    .unwrap();
    fs::write(
        dir.join("tests/input/cycle/b.py"),
        "from tests.input.cycle.a import alpha\n",
    )
    .unwrap();

    let files: Vec<_> = ["tests/input/cycle/a.py", "tests/input/cycle/b.py"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    assert!(detect(&cg, &["**/tests/**".to_string()]).is_empty());
}

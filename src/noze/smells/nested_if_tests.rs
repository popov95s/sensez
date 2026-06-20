use super::detect_local;
use crate::config::smells::Smells;
use crate::noze::SmellKind;
use crate::spine::parser::{parse_file, ParsedFile};
use std::fs;

fn parsed(name: &str, ext: &str, body: &str) -> ParsedFile {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(format!("{name}.{ext}"));
    fs::write(&path, body).unwrap();
    parse_file(&path, 0).unwrap()
}

fn has_nested_if(file: &ParsedFile) -> bool {
    detect_local(file, &Smells::default())
        .iter()
        .any(|finding| finding.kind == SmellKind::UnnecessaryNestedIf)
}

#[test]
fn python_flags_only_collapsible_nested_if() {
    let bad = parsed(
        "bad",
        "py",
        "def f(x, y):\n    if x:\n        if y:\n            return 1\n    return 0\n",
    );
    assert!(has_nested_if(&bad));

    let with_work = parsed(
        "with_work",
        "py",
        "def f(x, y):\n    if x:\n        prep()\n        if y:\n            return 1\n",
    );
    assert!(!has_nested_if(&with_work));

    let work_after_inner = parsed(
        "work_after_inner",
        "py",
        "def f(x, y):\n    if x:\n        if y:\n            save()\n        audit()\n",
    );
    assert!(!has_nested_if(&work_after_inner));

    let outer_else = parsed(
        "outer_else",
        "py",
        "def f(x, y):\n    if x:\n        if y:\n            return 1\n    else:\n        return 0\n",
    );
    assert!(!has_nested_if(&outer_else));

    let inner_else = parsed(
        "inner_else",
        "py",
        "def f(x, y):\n    if x:\n        if y:\n            return 1\n        else:\n            return 2\n",
    );
    assert!(!has_nested_if(&inner_else));
}

#[test]
fn javascript_flags_block_and_single_statement_forms() {
    let block = parsed(
        "block",
        "js",
        "function f(x, y) { if (x) { if (y) { return 1; } } return 0; }\n",
    );
    assert!(has_nested_if(&block));

    let single = parsed(
        "single",
        "js",
        "function f(x, y) { if (x) if (y) return 1; return 0; }\n",
    );
    assert!(has_nested_if(&single));

    let with_else = parsed(
        "with_else",
        "js",
        "function f(x, y) { if (x) { if (y) return 1; } else { return 0; } }\n",
    );
    assert!(!has_nested_if(&with_else));

    let work_after_inner = parsed(
        "work_after_inner",
        "js",
        "function f(x, y) { if (x) { if (y) { save(); } audit(); } }\n",
    );
    assert!(!has_nested_if(&work_after_inner));
}

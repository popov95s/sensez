use super::detect_local;
use crate::config::smells::Smells;
use crate::report::{Severity, SmellKind};
use crate::spine::parser::{parse_file, ParsedFile};
use std::fs;

fn parsed(name: &str, ext: &str, source: &str) -> ParsedFile {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(format!("{name}.{ext}"));
    fs::write(&path, source).unwrap();
    parse_file(&path, 0).unwrap()
}

fn nested(ext: &str, source: &str) -> Option<crate::report::SmellFinding> {
    let file = parsed("nested_ternary", ext, source);
    detect_local(&file, &Smells::default())
        .into_iter()
        .find(|finding| finding.kind == SmellKind::NestedTernary)
}

#[test]
fn python_counts_nested_ternaries_and_anchors_the_first_inner_expression() {
    let finding = nested(
        "py",
        "def status(a, b, c):\n    return 'a' if a else 'b' if b else 'c' if c else 'd'\n",
    )
    .expect("nested ternary must be reported");
    assert_eq!(finding.metric, 2);
    assert_eq!(finding.line, 2);
    assert_eq!(finding.severity, Severity::Warning);
    assert!(finding.message.contains("extract"));
    assert!(finding.message.contains("early returns"));
}

#[test]
fn multiline_nested_ternary_anchors_the_nested_expression_line() {
    let finding = nested(
        "py",
        "def status(a, b):\n    return (\n        'ready'\n        if a\n        else 'retry'\n        if b\n        else 'blocked'\n    )\n",
    )
    .expect("nested ternary must be reported");
    assert_eq!(finding.line, 5);
}

#[test]
fn javascript_flags_both_ternary_branches_but_not_standalone_ternaries() {
    let branch = nested(
        "js",
        "function status(a, b) { return a ? (b ? 'x' : 'y') : 'z'; }\n",
    );
    assert!(branch.is_some());

    let clean = nested(
        "js",
        "function status(a, b) { const x = a ? 'x' : 'y'; const z = b ? 'z' : 'q'; return x + z; }\n",
    );
    assert!(clean.is_none());
}

#[test]
fn nested_functions_do_not_inherit_outer_ternary_state() {
    let source =
        "function outer(a) { const value = a ? 'x' : 'y'; return () => value ? 'yes' : 'no'; }\n";
    assert!(nested("js", source).is_none());
}

#[test]
fn disabled_rule_suppresses_the_finding() {
    let file = parsed(
        "disabled",
        "py",
        "def status(a, b):\n    return 'x' if a else 'y' if b else 'z'\n",
    );
    let cfg = Smells {
        disabled: vec![SmellKind::NestedTernary],
        ..Smells::default()
    };
    assert!(detect_local(&file, &cfg)
        .into_iter()
        .all(|finding| finding.kind != SmellKind::NestedTernary));
}

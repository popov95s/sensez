use super::detect_local;
use super::test_support as fixture;
use crate::config::smells::{Smells, Strictness};
use crate::report::{Severity, SmellKind};
use crate::spine::parser::ParsedFile;

fn parsed(name: &str, ext: &str, source: &str) -> ParsedFile {
    fixture::parsed(&format!("{name}.{ext}"), source)
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
    assert_eq!(finding.metric, 3);
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

#[test]
fn ternaries_behind_expression_boundaries_are_not_chains() {
    for (ext, source) in [
        ("tsx", "export const NtJsx = (p: { open: boolean; c: number }) => <div>{p.open ? <Panel title={p.c > 0 ? \"Items\" : \"Empty\"} /> : null}</div>;\n"),
        ("ts", "export const ntArgument = (active: boolean, big: boolean) => (active ? cn(\"a\", big ? \"x\" : \"y\") : \"b\");\n"),
        ("ts", "export const ntTemplate = (done: boolean, n: number) => (done ? `${n} ${n === 1 ? \"file\" : \"files\"}` : \"pending\");\n"),
        ("ts", "export const ntSpread = (withMeta: boolean, draft: boolean) => ({ ...(withMeta ? { status: draft ? \"d\" : \"l\" } : {}) });\n"),
    ] {
        assert!(nested(ext, source).is_none(), "{source}");
    }
}

#[test]
fn direct_chains_keep_firing() {
    for (ext, source) in [
        (
            "ts",
            "export const grade = (s: number) => (s > 90 ? \"A\" : s > 50 ? \"B\" : \"C\");\n",
        ),
        (
            "ts",
            "export const odd = (a: boolean, b: boolean) => ((a ? b : !b) ? \"y\" : \"n\");\n",
        ),
        (
            "tsx",
            "export const View = (p: P) => <div>{p.a ? <A /> : p.b ? <B /> : null}</div>;\n",
        ),
        (
            "ts",
            "export const w = (a: boolean, b: boolean) => (a ? 1 + (b ? 1 : 2) : 0);\n",
        ),
    ] {
        assert!(nested(ext, source).is_some(), "{source}");
    }
}

#[test]
fn metric_is_the_deepest_chain_not_the_sum() {
    let finding = nested(
        "ts",
        "export function two(a: boolean, b: boolean, c: boolean, d: boolean) {\n  const x = a ? 1 : b ? 2 : 3;\n  const y = c ? 1 : d ? 2 : 3;\n  return x + y;\n}\n",
    )
    .expect("chains must be reported");
    assert_eq!(finding.metric, 2);
    assert!(
        finding
            .message
            .starts_with("2 nested ternary chain(s), deepest 2 levels"),
        "{}",
        finding.message
    );
}

#[test]
fn high_strictness_restores_the_lexical_rule() {
    let file = parsed(
        "lexical",
        "ts",
        "export const ntArgument = (active: boolean, big: boolean) => (active ? cn(\"a\", big ? \"x\" : \"y\") : \"b\");\n",
    );
    let cfg = Smells {
        nested_ternary_strictness: Strictness::High,
        ..Smells::default()
    };
    assert!(detect_local(&file, &cfg)
        .into_iter()
        .any(|finding| finding.kind == SmellKind::NestedTernary));
}

#[test]
fn python_argument_ternary_is_a_boundary() {
    let source = "def f(a, b):\n    return g('x' if b else 'y') if a else 'z'\n";
    assert!(nested("py", source).is_none());
}

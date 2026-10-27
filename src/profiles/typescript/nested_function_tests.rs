use crate::config::smells::Smells;
use crate::noze::smells::detect_local;
use crate::noze::smells::test_support as fixture;
use crate::report::{Severity, SmellFinding, SmellKind};

fn local_ts(source: &str, config: &Smells) -> Vec<SmellFinding> {
    let file = fixture::parsed("sample.ts", source);
    detect_local(&file, config)
}

fn heavy_nested(findings: &[SmellFinding]) -> Vec<&SmellFinding> {
    findings
        .iter()
        .filter(|finding| finding.kind == SmellKind::HeavyNestedFunction)
        .collect()
}

fn body(uses: &str) -> String {
    (0..20)
        .map(|i| format!("    total += {uses}{i};\n"))
        .collect()
}

#[test]
fn nested_helpers_in_test_framework_callbacks_are_skipped() {
    for callee in ["describe", "it.only", "test.each([1, 2])", "beforeEach"] {
        let source = format!(
            "{callee}(\"s\", () => {{\n  function mk(n: number) {{\n    let total = n;\n{}    return total;\n  }}\n  mk(1);\n}});\n",
            body("n + ")
        );
        let findings = local_ts(&source, &Smells::default());
        assert!(heavy_nested(&findings).is_empty(), "{callee}: {findings:?}");
    }
}

#[test]
fn self_contained_nested_function_stays_a_warning() {
    let source = format!(
        "function outer() {{\n  function parse(s: string) {{\n    let total = 0;\n{}    return total;\n  }}\n  return parse(\"x\");\n}}\n",
        body("s.length + ")
    );
    let findings = local_ts(&source, &Smells::default());
    let found = heavy_nested(&findings);
    assert_eq!(found.len(), 1, "{findings:?}");
    assert_eq!(found[0].severity, Severity::Warning);
    assert!(found[0].message.contains("`outer`"), "{}", found[0].message);
}

fn capturing_closure() -> String {
    format!(
        "function Comp(props: Props) {{\n  function onSave() {{\n    let total = 0;\n{}    return total;\n  }}\n  return onSave();\n}}\n",
        body("props.id + ")
    )
}

#[test]
fn capturing_closure_is_info() {
    let findings = local_ts(&capturing_closure(), &Smells::default());
    let found = heavy_nested(&findings);
    assert_eq!(found.len(), 1, "{findings:?}");
    assert_eq!(found[0].severity, Severity::Info);
    assert!(
        found[0].message.contains("explicit parameters"),
        "{}",
        found[0].message
    );
}

#[test]
fn skip_capturing_knob_silences_capturing_closure() {
    let skip = Smells {
        nested_function_skip_capturing: true,
        ..Smells::default()
    };
    let findings = local_ts(&capturing_closure(), &skip);
    assert!(heavy_nested(&findings).is_empty(), "{findings:?}");
}

#[test]
fn functions_a_factory_returns_are_not_heavy_nested() {
    let source = format!(
        "export function createCounter(start: number) {{\n  let total = start;\n  function bump(n: number) {{\n{}    return total;\n  }}\n  function reset() {{\n{}    return total;\n  }}\n  return {{ bump, reset: reset }};\n}}\n",
        body("n + "),
        body("start + ")
    );
    let findings = local_ts(&source, &Smells::default());
    assert!(heavy_nested(&findings).is_empty(), "{findings:?}");
}

#[test]
fn nested_arrow_const_is_heavy_nested_like_a_function() {
    let source = format!(
        "export function importRows(rows: string[]) {{\n  const normalize = (s: string) => {{\n    let total = 0;\n{}    return total;\n  }};\n  return rows.map(normalize);\n}}\n",
        body("s.length + ")
    );
    let findings = local_ts(&source, &Smells::default());
    assert_eq!(heavy_nested(&findings).len(), 1, "{findings:?}");
}

#[test]
fn handler_consts_inside_components_and_hooks_are_not_heavy_nested() {
    for parent in ["SaveButton", "useSave"] {
        let source = format!(
            "export function {parent}(props: Props) {{\n  const onSave = async () => {{\n    let total = 0;\n{}    return total;\n  }};\n  return onSave();\n}}\n",
            body("props.id + ")
        );
        let findings = local_ts(&source, &Smells::default());
        assert!(heavy_nested(&findings).is_empty(), "{parent}: {findings:?}");
    }
}

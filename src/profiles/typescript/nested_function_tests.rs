use crate::config::smells::Smells;
use crate::noze::smells::detect_local;
use crate::report::{Severity, SmellFinding, SmellKind};
use crate::spine::parser::parse_file;
use std::fs;

fn local_ts(source: &str, config: &Smells) -> Vec<SmellFinding> {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sample.ts");
    fs::write(&path, source).unwrap();
    let file = parse_file(&path, 0).unwrap();
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

#[test]
fn capturing_closure_is_info_or_skipped_by_knob() {
    let source = format!(
        "function Comp(props: Props) {{\n  function onSave() {{\n    let total = 0;\n{}    return total;\n  }}\n  return onSave;\n}}\n",
        body("props.id + ")
    );
    let findings = local_ts(&source, &Smells::default());
    let found = heavy_nested(&findings);
    assert_eq!(found.len(), 1, "{findings:?}");
    assert_eq!(found[0].severity, Severity::Info);
    assert!(found[0].message.contains("explicit parameters"));

    let skip = Smells {
        nested_function_skip_capturing: true,
        ..Smells::default()
    };
    assert!(heavy_nested(&local_ts(&source, &skip)).is_empty());
}

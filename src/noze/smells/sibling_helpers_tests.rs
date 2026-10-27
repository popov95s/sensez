use super::sibling_helpers;
use super::test_support::project;
use crate::config::smells::SmellConfig;
use crate::report::SmellFinding;

const REASON_OF: &str = "export function reasonOf(failure: unknown) {\n  return failure instanceof Error ? failure.message : String(failure);\n}\n";
const REASON_OF_RENAMED: &str = "function reasonOf(cause: unknown) {\n  return cause instanceof Error ? cause.message : String(cause);\n}\n";

fn findings(files: &[(&str, &str)]) -> Vec<SmellFinding> {
    let (parsed, graph) = project(files);
    let kept: Vec<_> = parsed.iter().collect();
    sibling_helpers::detect(&kept, &graph, &SmellConfig::default())
}

#[test]
fn copied_helper_fires_once_at_the_first_definition() {
    let found = findings(&[("a/format.ts", REASON_OF), ("b/log.ts", REASON_OF_RENAMED)]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].file.ends_with("a/format.ts"));
    assert_eq!(found[0].line, 1);
    assert!(
        found[0].message.contains("b/log.ts:L1"),
        "{}",
        found[0].message
    );
}

#[test]
fn every_copy_joins_one_finding() {
    let found = findings(&[
        ("a/format.ts", REASON_OF),
        ("b/log.ts", REASON_OF_RENAMED),
        ("c/report.ts", REASON_OF_RENAMED),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].metric, 3);
}

#[test]
fn same_name_with_a_different_body_is_silent() {
    let other = "function reasonOf(cause: unknown) {\n  const text = JSON.stringify({ cause, at: Date.now() });\n  console.warn(text);\n  return text.trim();\n}\n";
    assert!(findings(&[("a/format.ts", REASON_OF), ("b/log.ts", other)]).is_empty());
}

#[test]
fn a_module_importing_the_name_is_a_wrapper() {
    let wrapper = format!("import {{ reasonOf }} from \"../a/format\";\n{REASON_OF_RENAMED}");
    assert!(findings(&[("a/format.ts", REASON_OF), ("b/log.ts", &wrapper)]).is_empty());
}

#[test]
fn a_different_arity_is_silent() {
    let two = "function reasonOf(cause: unknown, fallback: string) {\n  return cause instanceof Error ? cause.message : String(cause);\n}\n";
    assert!(findings(&[("a/format.ts", REASON_OF), ("b/log.ts", two)]).is_empty());
}

#[test]
fn pascal_case_names_are_silent() {
    let component = REASON_OF.replace("reasonOf", "ReasonOf");
    let copy = REASON_OF_RENAMED.replace("reasonOf", "ReasonOf");
    assert!(findings(&[("a/format.ts", &component), ("b/log.ts", &copy)]).is_empty());
}

#[test]
fn framework_vocabulary_is_silent() {
    let first = REASON_OF.replace("reasonOf", "loader");
    let second = REASON_OF_RENAMED.replace("reasonOf", "loader");
    assert!(findings(&[("a/format.ts", &first), ("b/log.ts", &second)]).is_empty());
}

#[test]
fn test_sources_are_silent() {
    assert!(findings(&[
        ("a/format.ts", REASON_OF),
        ("b/log.test.ts", REASON_OF_RENAMED)
    ])
    .is_empty());
}

#[test]
fn package_index_files_are_silent() {
    assert!(findings(&[
        ("a/format.ts", REASON_OF),
        ("b/index.ts", REASON_OF_RENAMED)
    ])
    .is_empty());
}

#[test]
fn bodies_below_the_lexeme_floor_are_silent() {
    let tiny = "function id(x: number) { return x; }\n";
    assert!(findings(&[("a/format.ts", tiny), ("b/log.ts", tiny)]).is_empty());
}

#[test]
fn python_copies_fire_too() {
    let body = "def reason_of(failure):\n    return str(failure.args[0]) if failure.args else repr(failure)\n";
    let copy = "def reason_of(err):\n    return str(err.args[0]) if err.args else repr(err)\n";
    assert_eq!(
        findings(&[("a/format.py", body), ("b/log.py", copy)]).len(),
        1
    );
}

#[test]
fn a_forked_module_copy_is_left_to_the_clone_pillar() {
    assert!(findings(&[
        ("v1/shared/format.ts", REASON_OF),
        ("v2/shared/format.ts", REASON_OF_RENAMED)
    ])
    .is_empty());
}

#[test]
fn module_script_tests_are_silent() {
    let script = "export function reasonOf(cause) {\n  return cause instanceof Error ? cause.message : String(cause);\n}\n";
    assert!(findings(&[("a/format.mjs", script), ("b/log.test.mjs", script)]).is_empty());
}

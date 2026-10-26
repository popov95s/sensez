//! What counts as a test oracle.

use super::test_support::local_of;
use crate::config::smells::Smells;
use crate::report::SmellKind;

fn weak_oracle_lines(source: &str, config: &Smells) -> Vec<usize> {
    local_of("oracle.test.ts", source, config, SmellKind::WeakTestOracle)
        .into_iter()
        .map(|finding| finding.line)
        .collect()
}

fn oracle_config(assertion_functions: &[&str]) -> Smells {
    let mut config = Smells::default();
    config
        .disabled
        .retain(|kind| *kind != SmellKind::WeakTestOracle);
    config.assertion_functions = assertion_functions.iter().map(|s| s.to_string()).collect();
    config
}

fn weak(source: &str) -> Vec<usize> {
    weak_oracle_lines(source, &oracle_config(&[]))
}

#[test]
fn helper_that_calls_a_checking_helper_is_an_oracle() {
    let source = r#"
async function checkRejected(p: Promise<unknown>) {
  await expect(p).rejects.toThrow();
}
async function mustReject(p: Promise<unknown>) {
  await checkRejected(p);
}
it("denies", async () => {
  await mustReject(call());
});
"#;
    assert_eq!(weak(source), Vec::<usize>::new());
}

#[test]
fn helper_chain_four_deep_is_an_oracle() {
    let source = r#"
function d(x: number) { expect(x).toBe(1); }
function c(x: number) { d(x); }
function b(x: number) { c(x); }
function a(x: number) { b(x); }
it("chains", () => {
  a(call());
});
"#;
    assert_eq!(weak(source), Vec::<usize>::new());
}

#[test]
fn expect_type_of_is_an_oracle() {
    let source = "it(\"types\", () => {\n  expectTypeOf(call).toBeFunction();\n});\n";
    assert_eq!(weak(source), Vec::<usize>::new());
}

#[test]
fn assert_type_is_an_oracle() {
    let source = "it(\"asserts types\", () => {\n  assertType<string>(call());\n});\n";
    assert_eq!(weak(source), Vec::<usize>::new());
}

#[test]
fn expect_assertions_count_is_an_oracle() {
    let source = "it(\"counts\", async () => {\n  expect.assertions(1);\n  await call();\n});\n";
    assert_eq!(weak(source), Vec::<usize>::new());
}

#[test]
fn focused_test_without_checks_is_weak() {
    let source = "it.only(\"really weak\", async () => {\n  await call();\n});\n";
    assert_eq!(weak(source), vec![1]);
}

#[test]
fn assertion_function_globs_credit_imported_helpers() {
    let source = "it(\"verifies\", () => {\n  verifyUser(load());\n});\n";
    assert!(weak_oracle_lines(source, &oracle_config(&["verify*"])).is_empty());
    assert_eq!(weak(source).len(), 1);
}

#[test]
fn bare_expect_is_weak_even_with_default_globs() {
    let defaults = oracle_config(&["expect*", "assert*", "verify*"]);
    let bare = "it(\"x\", () => {\n  expect(call());\n});\n";
    assert_eq!(weak_oracle_lines(bare, &defaults), vec![1]);
}

#[test]
fn expect_prefixed_helper_matches_default_globs() {
    let defaults = oracle_config(&["expect*", "assert*", "verify*"]);
    let helper = "it(\"x\", async () => {\n  await expectRejected(call());\n});\n";
    assert!(weak_oracle_lines(helper, &defaults).is_empty());
}

//! Assertions that cannot fail.

use super::test_support::local_of;
use crate::config::smells::Smells;
use crate::report::{Severity, SmellKind};

fn echoes(name: &str, source: &str, config: &Smells) -> Vec<(usize, Severity)> {
    local_of(name, source, config, SmellKind::FixtureEchoAssertion)
        .into_iter()
        .map(|finding| (finding.line, finding.severity))
        .collect()
}

fn ts(source: &str) -> Vec<(usize, Severity)> {
    echoes("echo.test.ts", source, &Smells::default())
}

fn with_mock_only(source: &str) -> Vec<(usize, Severity)> {
    let config = Smells {
        fixture_echo_include_mock_only: true,
        ..Smells::default()
    };
    echoes("echo.test.ts", source, &config)
}

fn py(source: &str) -> Vec<(usize, Severity)> {
    echoes("test_echo.py", source, &Smells::default())
}

#[test]
fn same_member_on_both_sides_is_flagged() {
    let source = "it(\"keeps\", () => {\n  const user = makeUser();\n  expect(user.name).toBe(user.name);\n});\n";
    assert_eq!(ts(source), vec![(3, Severity::Warning)]);
}

#[test]
fn equal_literals_are_flagged() {
    let source = "it(\"runs\", () => {\n  run();\n  expect(1).toEqual(1);\n});\n";
    assert_eq!(ts(source), vec![(3, Severity::Warning)]);
}

#[test]
fn field_read_through_unchanged_alias_is_flagged() {
    let source = "it(\"copies\", () => {\n  const input = makeUser();\n  const result = input;\n  expect(result.name).toBe(input.name);\n});\n";
    assert_eq!(ts(source), vec![(4, Severity::Warning)]);
}

#[test]
fn result_of_the_subject_is_not_flagged() {
    let source = "it(\"renames\", () => {\n  const user = makeUser();\n  expect(rename(user).name).toBe(user.name);\n});\n";
    assert!(ts(source).is_empty());
}

#[test]
fn computed_copy_is_not_an_alias() {
    let source = "it(\"copies\", () => {\n  const input = makeUser();\n  const result = copy(input);\n  expect(result.name).toBe(input.name);\n});\n";
    assert!(ts(source).is_empty());
}

#[test]
fn repeated_call_checks_determinism() {
    let source = "it(\"is stable\", () => {\n  expect(hash(seed)).toBe(hash(seed));\n});\n";
    assert!(ts(source).is_empty());
}

#[test]
fn negated_matcher_is_not_flagged() {
    let source = "it(\"differs\", () => {\n  expect(a).not.toBe(a);\n});\n";
    assert!(ts(source).is_empty());
}

#[test]
fn node_assert_equal_with_same_sides_is_flagged() {
    let source = "test(\"same\", () => {\n  assert.equal(total, total);\n});\n";
    assert_eq!(ts(source), vec![(2, Severity::Warning)]);
}

#[test]
fn only_checking_own_programmed_mock_is_info() {
    let source = "it(\"loads\", async () => {\n  const fetchUser = vi.fn();\n  fetchUser.mockResolvedValue(user);\n  await load(fetchUser);\n  expect(fetchUser).toHaveBeenCalled();\n});\n";
    assert_eq!(with_mock_only(source), vec![(1, Severity::Info)]);
}

#[test]
fn interaction_check_on_an_unprogrammed_callback_is_not_flagged() {
    let source = "it(\"clicks\", () => {\n  const onSave = vi.fn();\n  click(onSave);\n  expect(onSave).toHaveBeenCalled();\n});\n";
    assert!(with_mock_only(source).is_empty());
}

#[test]
fn called_with_arguments_is_not_flagged() {
    let source = "it(\"loads\", async () => {\n  const fetchUser = vi.fn().mockResolvedValue(user);\n  await load(fetchUser);\n  expect(fetchUser).toHaveBeenCalledWith(7);\n});\n";
    assert!(with_mock_only(source).is_empty());
}

#[test]
fn mock_check_beside_a_value_check_is_not_flagged() {
    let source = "it(\"loads\", async () => {\n  const fetchUser = vi.fn();\n  const loaded = await load(fetchUser);\n  expect(fetchUser).toHaveBeenCalled();\n  expect(loaded.id).toBe(7);\n});\n";
    assert!(with_mock_only(source).is_empty());
}

#[test]
fn mock_from_outside_the_test_is_not_flagged() {
    let source = "const save = vi.fn().mockReturnValue(1);\nit(\"saves\", () => {\n  submit(save);\n  expect(save).toHaveBeenCalled();\n});\n";
    assert!(with_mock_only(source).is_empty());
}

#[test]
fn mock_only_variant_is_off_by_default() {
    let source = "it(\"calls\", () => {\n  const done = vi.fn().mockReturnValue(1);\n  run(done);\n  expect(done).toHaveBeenCalled();\n});\n";
    assert!(ts(source).is_empty());
}

#[test]
fn call_count_check_is_not_flagged() {
    let source = "it(\"retries\", async () => {\n  const send = vi.fn().mockRejectedValueOnce(new Error()).mockResolvedValue(1);\n  await deliver(send);\n  expect(send).toHaveBeenCalledTimes(2);\n});\n";
    assert!(with_mock_only(source).is_empty());
}

#[test]
fn checks_in_a_nested_callback_count_as_value_checks() {
    let source = "it(\"waits\", async () => {\n  const done = vi.fn().mockReturnValue(1);\n  run(done);\n  expect(done).toHaveBeenCalled();\n  await waitFor(() => {\n    expect(screen.text).toBe(\"ok\");\n  });\n});\n";
    assert!(with_mock_only(source).is_empty());
}

#[test]
fn non_test_source_is_ignored() {
    let source = "export function check(a: number) {\n  expect(a).toBe(a);\n}\n";
    assert!(echoes("check.ts", source, &Smells::default()).is_empty());
}

#[test]
fn python_same_sides_is_flagged() {
    let source = "def test_keeps():\n    user = make_user()\n    assert user.name == user.name\n";
    assert_eq!(py(source), vec![(3, Severity::Warning)]);
}

#[test]
fn python_assert_true_is_flagged() {
    let source = "def test_runs():\n    run()\n    assert True\n";
    assert_eq!(py(source), vec![(3, Severity::Warning)]);
}

#[test]
fn python_unittest_equal_on_alias_is_flagged() {
    let source = "class T:\n    def test_copies(self):\n        original = make_user()\n        result = original\n        self.assertEqual(result.name, original.name)\n";
    assert_eq!(py(source), vec![(5, Severity::Warning)]);
}

#[test]
fn python_not_equal_is_not_flagged() {
    let source = "def test_differs():\n    assert a != a\n";
    assert!(py(source).is_empty());
}

#[test]
fn python_result_of_the_subject_is_not_flagged() {
    let source = "def test_sorts():\n    assert sorted(xs) == xs\n";
    assert!(py(source).is_empty());
}

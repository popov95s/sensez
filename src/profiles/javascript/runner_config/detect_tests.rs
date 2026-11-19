use super::*;
use serde_json::json;

#[test]
fn a_script_mentioning_a_runner_as_an_argument_does_not_select_it() {
    let manifest =
        json!({"scripts": {"lint": "eslint --plugin jest ."}, "devDependencies": {"jest": "1"}});

    assert_eq!(manifest_runners(&manifest), [RunnerKind::Jest]);
    let manifest = json!({"scripts": {"lint": "eslint --plugin jest ."}});
    assert!(manifest_runners(&manifest).is_empty());
}

#[test]
fn scripts_invoking_both_runners_select_both() {
    let manifest =
        json!({"scripts": {"test": "vitest", "test:e2e": "cross-env CI=1 jest -c e2e.config.js"}});

    assert_eq!(
        manifest_runners(&manifest),
        [RunnerKind::Vitest, RunnerKind::Jest]
    );
}

#[test]
fn a_runner_binary_path_counts_as_invoking_it() {
    let manifest = json!({"scripts": {"test": "node --experimental-vm-modules node_modules/jest/bin/jest.js"}});

    assert_eq!(manifest_runners(&manifest), [RunnerKind::Jest]);
}

#[test]
fn dependencies_prefer_vitest_without_scripts() {
    let manifest = json!({"devDependencies": {"jest": "1", "vitest": "1"}});

    assert_eq!(manifest_runners(&manifest), [RunnerKind::Vitest]);
}

#[test]
fn vite_config_with_a_test_key_is_a_vitest_project() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("vite.config.ts"),
        "export default { test: { include: ['src/**/*.test.ts'] } };\n",
    )
    .unwrap();

    assert_eq!(config_runner(root.path()), Some(RunnerKind::Vitest));
}

#[test]
fn vite_config_without_a_test_key_is_not_a_runner() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("vite.config.ts"),
        "export default { base: '/' };\n",
    )
    .unwrap();

    assert_eq!(config_runner(root.path()), None);
}

#[test]
fn vitest_workspace_file_is_a_vitest_project() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("vitest.workspace.ts"),
        "export default ['packages/*'];\n",
    )
    .unwrap();

    assert_eq!(config_runner(root.path()), Some(RunnerKind::Vitest));
}

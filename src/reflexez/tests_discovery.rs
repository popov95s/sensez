use super::model::RunnerKind;
use super::tests::{args, Project};
use crate::cli::spec::ReflexezArgs;

fn full() -> ReflexezArgs {
    ReflexezArgs {
        full: true,
        ..args()
    }
}

#[test]
fn unparsable_package_json_is_a_warning() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    project.change("tests/fixtures/bad/package.json", "{ /* comment */ }");

    let plan = project.plan();

    assert_eq!(plan.config_warnings.len(), 1);
    assert!(plan.config_warnings[0]
        .file
        .ends_with("tests/fixtures/bad/package.json"));
}

#[test]
fn runner_project_without_tests_is_not_run() {
    let project = Project::new("import './feature'; test('feature', () => 1);\n");
    project.change(
        "examples/demo/package.json",
        r#"{"devDependencies":{"jest":"29"}}"#,
    );
    project.change("examples/demo/index.js", "module.exports = 1;\n");

    let plan = project.plan_with(full());

    let roots: Vec<_> = plan
        .runners
        .iter()
        .map(|runner| (runner.kind, project.relative(&runner.root)))
        .collect();
    assert_eq!(roots, [(RunnerKind::Vitest, String::new())]);
}

#[test]
fn nested_pytest_config_sets_runner_root_and_patterns() {
    let project = Project::with_files(&[
        (
            "backend/pyproject.toml",
            "[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\npython_files = [\"check_*.py\", \"test_*.py\"]\n",
        ),
        ("backend/app/math.py", "def add(a, b):\n    return a + b\n"),
        ("backend/tests/check_math.py", "from app.math import add\n"),
        ("backend/tests/test_other.py", "def test_other():\n    pass\n"),
        ("backend/scripts/test_script.py", "def test_script():\n    pass\n"),
        ("web/a.ts", "export const a = 1;\n"),
        ("web/a.test.ts", "import { a } from './a'; test('a', () => a);\n"),
    ]);

    let plan = project.plan_with(full());

    let pytest: Vec<_> = plan
        .selected
        .iter()
        .filter(|selection| selection.runner == RunnerKind::Pytest)
        .map(|selection| project.relative(&selection.file))
        .collect();
    assert_eq!(
        pytest,
        ["backend/tests/check_math.py", "backend/tests/test_other.py"]
    );
    let pytest_root = plan
        .runners
        .iter()
        .find(|runner| runner.kind == RunnerKind::Pytest)
        .map(|runner| project.relative(&runner.root));
    assert_eq!(pytest_root.as_deref(), Some("backend"));
}

use super::*;
use crate::test_support::write;

#[test]
fn reads_pyproject_testpaths() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "pyproject.toml",
        "[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n",
    );
    let mut configs = PytestConfigs::new(root.path());

    let config = configs.for_file(&root.path().join("tests/test_one.py"));

    assert!(config.collects(&root.path().join("tests/test_one.py")));
    assert!(!config.collects(&root.path().join("examples/test_one.py")));
}

#[test]
fn reads_multiline_ini_testpaths() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "pytest.ini",
        "[pytest]\ntestpaths =\n    tests\n    integration\n",
    );
    let mut configs = PytestConfigs::new(root.path());

    let config = configs.for_file(&root.path().join("integration/test_one.py"));

    assert!(config.collects(&root.path().join("integration/test_one.py")));
    assert!(config.collects(&root.path().join("tests/test_two.py")));
}

#[test]
fn default_python_files_patterns_mark_test_files() {
    let root = tempfile::tempdir().unwrap();
    let mut configs = PytestConfigs::new(root.path());
    let config = configs.for_file(&root.path().join("tests/test_api.py"));

    assert!(config.collects(&root.path().join("tests/test_api.py")));
    assert!(config.collects(&root.path().join("pkg/api_test.py")));
    assert!(!config.collects(&root.path().join("tests/conftest.py")));
    assert!(!config.collects(&root.path().join("web/test_api.ts")));
}

#[test]
fn nested_pyproject_sets_rootdir_testpaths_and_python_files() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "backend/pyproject.toml",
        "[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\npython_files = [\"check_*.py\", \"test_*.py\"]\n",
    );
    let mut configs = PytestConfigs::new(root.path());
    let check = root.path().join("backend/tests/check_math.py");
    let script = root.path().join("backend/scripts/test_script.py");

    let config = configs.for_file(&check);

    assert_eq!(config.dir, root.path().join("backend"));
    assert!(config.collects(&check));
    assert!(!configs.for_file(&script).collects(&script));
}

#[test]
fn pyproject_without_pytest_section_is_not_a_rootdir() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "backend/pyproject.toml",
        "[project]\nname = \"x\"\n",
    );
    let mut configs = PytestConfigs::new(root.path());

    let config = configs.for_file(&root.path().join("backend/tests/test_a.py"));

    assert_eq!(config.dir, root.path());
}

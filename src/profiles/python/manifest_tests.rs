use super::*;
use std::fs;

#[test]
fn pyproject_sections_map_to_scopes() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join(PYPROJECT),
        "[project]\ndependencies = [\"More_Itertools>=10\"]\n[project.optional-dependencies]\ntest = [\"tenacity\"]\n[tool.poetry.dependencies]\npython = \"^3.12\"\ntoolz = \"*\"\n",
    )
    .unwrap();

    let declared = declared_packages(tmp.path());

    assert!(declared
        .origin("more-itertools", DependencyScope::Production)
        .is_some());
    assert!(declared
        .origin("toolz", DependencyScope::Production)
        .is_some());
    assert!(declared
        .origin("python", DependencyScope::Production)
        .is_none());
    assert!(declared
        .origin("tenacity", DependencyScope::Production)
        .is_none());
    assert!(declared
        .origin("tenacity", DependencyScope::Development)
        .is_some());
}

#[test]
fn requirement_files_skip_options_and_mark_dev_files() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("requirements.txt"),
        "# pinned\n-r base.txt\nbackoff==2.2 # retries\n",
    )
    .unwrap();
    fs::write(tmp.path().join("requirements-dev.txt"), "toolz\n").unwrap();

    let declared = declared_packages(tmp.path());

    assert!(declared
        .origin("backoff", DependencyScope::Production)
        .is_some());
    assert!(declared
        .origin("toolz", DependencyScope::Production)
        .is_none());
    assert_eq!(declared.production.len(), 1);
}

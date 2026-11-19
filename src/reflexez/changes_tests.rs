use super::*;

#[test]
fn recognizes_configuration_and_shared_setup_triggers() {
    for path in [
        "sensez.toml",
        "pyproject.toml",
        "vitest.config.ts",
        "tests/test_setup.py",
        ".env.test.local",
    ] {
        assert!(global_trigger(Path::new(path)).is_some(), "{path}");
    }
    assert!(global_trigger(Path::new("src/setup.ts")).is_none());
}

#[test]
fn deleted_non_source_file_is_not_a_fallback() {
    let deleted = [PathBuf::from("README.md"), PathBuf::from("assets/note.txt")];

    assert!(fallback_reasons(&deleted, &deleted).is_empty());
}

#[test]
fn deleted_source_file_is_a_fallback() {
    let deleted = [PathBuf::from("src/math.ts")];

    assert_eq!(fallback_reasons(&deleted, &deleted).len(), 1);
}

#[test]
fn changed_file_is_relative_to_the_working_directory() {
    let repository = tempfile::tempdir().unwrap();
    let repository = std::fs::canonicalize(repository.path()).unwrap();
    std::fs::create_dir_all(repository.join("backend/app")).unwrap();
    std::fs::write(repository.join("backend/app/math.py"), "").unwrap();

    let file = explicit_file(
        Path::new("app/math.py"),
        &repository.join("backend"),
        &repository,
    );

    assert_eq!(file, repository.join("backend/app/math.py"));
}

#[test]
fn changed_file_missing_under_cwd_is_relative_to_the_repository() {
    let repository = tempfile::tempdir().unwrap();
    let repository = std::fs::canonicalize(repository.path()).unwrap();
    std::fs::create_dir_all(repository.join("backend")).unwrap();

    let file = explicit_file(
        Path::new("src/gone.ts"),
        &repository.join("backend"),
        &repository,
    );

    assert_eq!(file, repository.join("src/gone.ts"));
}

#[test]
fn sources_are_python_javascript_and_typescript_files() {
    for path in ["app/views.py", "web/a.mjs", "web/b.tsx", "web/c.cts"] {
        assert!(is_source(Path::new(path)), "{path}");
    }
    for path in ["src/lib.rs", "README.md", "data.json"] {
        assert!(!is_source(Path::new(path)), "{path}");
    }
}

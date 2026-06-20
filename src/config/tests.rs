//! Config loading tests.

use super::*;

#[test]
fn defaults_when_missing() {
    let cfg = Config::load(Path::new("/nonexistent/xyz")).unwrap();
    assert_eq!(cfg.duplication.threshold, 50);
    assert!(
        cfg.dead_code.entrypoints.is_empty(),
        "language-specific dead-code defaults are profile-scoped, not global config"
    );
}

/// The test/migration baseline is applied to duplication/smell exclusions even
/// when a config is minimal. Dead-code entry-point globs are profile-scoped
/// instead, so Python runner conventions do not leak into JS/TS/Rust config.
#[test]
fn baseline_excludes_survive_empty_config() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("sensez.toml"),
        "[dead_code]\nentry_points = []\n[duplication]\nthreshold = 50\n",
    )
    .unwrap();

    let cfg = Config::load(&dir).unwrap();
    assert!(cfg.dead_code.entry_points.is_empty());
    assert!(cfg.duplication.exclude.contains(&"**/tests/**".to_string()));
    assert!(cfg.exclude.contains(&"**/vendor/**".to_string()));
    assert!(cfg.exclude.contains(&"**/*.min.js".to_string()));
}

#[test]
fn invalid_globs_fail_loudly() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("sensez.toml"), "exclude = [\"[invalid\"]\n").unwrap();

    let err = Config::load(&dir).unwrap_err();
    assert!(
        err.to_string().contains("invalid glob in exclude"),
        "{err:#}"
    );
}

/// `[tool.sensez]` in pyproject.toml configures sensez when sensez.toml is absent;
/// sensez.toml wins when both exist.
#[test]
fn pyproject_tool_sensez_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("pyproject.toml"),
        "[project]\nname = \"x\"\n[tool.sensez.duplication]\nthreshold = 33\n\
         [tool.sensez.self_improvement]\nenabled = false\n",
    )
    .unwrap();

    let cfg = Config::load(&dir).unwrap();
    assert_eq!(cfg.duplication.threshold, 33);
    assert!(!cfg.self_improvement.enabled);

    // sensez.toml takes precedence over pyproject.
    std::fs::write(dir.join("sensez.toml"), "[duplication]\nthreshold = 44\n").unwrap();
    let cfg = Config::load(&dir).unwrap();
    assert_eq!(cfg.duplication.threshold, 44);
    assert!(
        cfg.self_improvement.enabled,
        "sensez.toml omits it -> default on"
    );
}

#[test]
fn action_policy_parses_pillars_and_smells() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("sensez.toml"),
        "[action]\ndead_code = \"info\"\nboundaries = \"must_fix\"\n\
         [action.smells]\nlong_function = \"must_fix\"\nmagic_numbers = \"info\"\n",
    )
    .unwrap();

    let cfg = Config::load(&dir).unwrap();
    assert_eq!(cfg.action.dead_code, crate::report::ActionLevel::Info);
    assert_eq!(cfg.action.boundaries, crate::report::ActionLevel::MustFix);
    assert_eq!(
        cfg.action.smells[&crate::report::SmellKind::LongFunction],
        crate::report::ActionLevel::MustFix
    );
    assert_eq!(
        cfg.action.smells[&crate::report::SmellKind::MagicNumbers],
        crate::report::ActionLevel::Info
    );
}

#[test]
fn gate_repeat_limit_defaults_and_parses() {
    assert_eq!(Config::default().gate.repeat_limit, 5);

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("sensez.toml"), "[gate]\nrepeat_limit = 3\n").unwrap();

    let cfg = Config::load(&dir).unwrap();
    assert_eq!(cfg.gate.repeat_limit, 3);
}

#[test]
fn signature_is_stable_and_changes_with_knobs() {
    let cfg = Config::default();
    assert_eq!(cfg.signature(), cfg.signature());

    let mut changed = cfg.clone();
    changed.duplication.threshold += 1;
    assert_ne!(cfg.signature(), changed.signature());

    let mut action_changed = cfg.clone();
    action_changed.action.smells.insert(
        crate::report::SmellKind::LongFunction,
        crate::report::ActionLevel::Info,
    );
    assert_ne!(cfg.signature(), action_changed.signature());
}

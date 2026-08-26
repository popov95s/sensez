//! Config loading tests.

use super::*;

#[test]
fn defaults_when_missing() {
    let cfg = Config::load(Path::new("/nonexistent/xyz")).unwrap();
    assert_eq!(cfg.duplication.threshold, 50);
    assert!(!cfg.cache.enabled, "analysis cache must be opt-in");
    assert!(
        !cfg.duplication.class_name_duplicates,
        "same-name class duplication is disabled by default"
    );
    assert_eq!(
        cfg.duplication.class_property_overlap_min, 4,
        "class-property overlap stays enabled by default"
    );
    assert!(
        !cfg.duplication.semantic.enabled,
        "semantic duplication stays opt-in until the detector is enabled"
    );
    assert!(
        cfg.duplication.semantic.comment_required,
        "comment-backed semantic duplication requires comments by default"
    );
    assert!(
        cfg.dead_code.entrypoints.is_empty(),
        "language-specific dead-code defaults are profile-scoped, not global config"
    );
}

#[test]
fn cache_config_is_opt_in() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("sensez.toml"), "[cache]\nenabled = true\n").unwrap();

    assert!(Config::load(tmp.path()).unwrap().cache.enabled);
}

#[test]
fn semantic_duplication_config_parses() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("sensez.toml"),
        "[duplication.semantic]\n\
         enabled = true\n\
         min_shape_score = 84\n\
         comment_boost_score = 91\n",
    )
    .unwrap();

    let cfg = Config::load(&dir).unwrap();
    assert!(cfg.duplication.semantic.enabled);
    assert_eq!(cfg.duplication.semantic.min_shape_score, 84);
    assert_eq!(cfg.duplication.semantic.comment_boost_score, 91);
    assert!(
        cfg.duplication.semantic.comment_required,
        "omitting comment_required keeps the safe default"
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
    assert!(cfg.exclude.contains(&"**/docs/**".to_string()));
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
    assert_eq!(Config::default().gate.repeat_limit, 2);

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

    let mut cache_changed = cfg.clone();
    cache_changed.cache.enabled = true;
    assert_ne!(cfg.signature(), cache_changed.signature());
}

#[test]
fn unknown_keys_warn_without_failing() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::write(
        dir.join("sensez.toml"),
        "[duplication]\ntreshold = 40\nthreshold = 60\n",
    )
    .unwrap();

    // Public load succeeds; the typo is ignored and the real knob applies.
    let cfg = Config::load(&dir).unwrap();
    assert_eq!(cfg.duplication.threshold, 60);

    // Scan loading surfaces the same problem as a ScanIssue.
    let (cfg, issues) = Config::load_for_scan(&dir);
    assert_eq!(cfg.duplication.threshold, 60);
    let unknown: Vec<_> = issues
        .iter()
        .filter(|issue| issue.message.contains("unknown config key"))
        .collect();
    assert_eq!(unknown.len(), 1);
    assert!(
        unknown[0].message.contains("duplication.treshold"),
        "{}",
        unknown[0].message
    );
}

#[test]
fn known_config_has_no_unknown_key_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::write(
        dir.join("sensez.toml"),
        "exclude = [\"**/generated/**\"]\n\
         [cache]\nenabled = false\n\
         [duplication]\nthreshold = 55\n\
         [duplication.semantic]\nenabled = false\ncomment_required = true\n\
         [dead_code]\nunused_methods = true\nentrypoints = [\"**/main.py\"]\n\
         [[boundaries.forbidden]]\nfrom = \"a\"\nto = \"b\"\n\
         [action]\ncycles = \"warning\"\n\
         [gate]\nrepeat_limit = 2\n\
         [self_improvement]\nenabled = true\n",
    )
    .unwrap();

    let (_, issues) = Config::load_for_scan(&dir);
    assert!(
        !issues
            .iter()
            .any(|issue| issue.message.contains("unknown config key")),
        "{issues:?}"
    );
}

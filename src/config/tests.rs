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
        !cfg.duplication.semantic.comment_required,
        "semantic duplication compares undocumented functions by default"
    );
    assert!(
        cfg.dead_code.registration_decorators.is_empty(),
        "language-specific dead-code defaults are profile-scoped, not global config"
    );
}

#[test]
fn cache_config_is_opt_in() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("sensez.toml"), "[cache]\nenabled = true\n").unwrap();

    assert!(Config::load(tmp.path()).unwrap().cache.enabled);
}

/// Migration globs join pillar excludes even for a minimal config; test layouts never do.
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
    assert!(cfg.dead_code.entry_globs.is_empty());
    assert!(!cfg.duplication.exclude.contains(&"**/tests/**".to_string()));
    assert!(!cfg.smells.exclude.contains(&"**/tests/**".to_string()));
    assert!(!cfg.cycles.exclude.contains(&"**/tests/**".to_string()));
    assert!(cfg.cycles.exclude.contains(&"**/migrations/**".to_string()));
    assert!(cfg
        .duplication
        .exclude
        .contains(&"**/versions/**/*.py".to_string()));
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

#[test]
fn invalid_boundary_glob_is_a_config_error() {
    let tmp = write_config("[[boundaries.forbidden]]\nfrom = \"core.[x\"\nto = \"web\"\n");

    let err = Config::load(tmp.path()).unwrap_err();
    assert!(
        err.to_string()
            .contains("invalid glob in boundaries.forbidden"),
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

fn write_config(text: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("sensez.toml"), text).unwrap();
    tmp
}

fn problems(text: &str) -> Vec<String> {
    let tmp = write_config(text);
    let errors = Config::load_with_warnings(tmp.path()).unwrap_err();
    errors.0.iter().map(ToString::to_string).collect()
}

#[test]
fn unknown_keys_outside_smells_and_action_warn_once_with_location() {
    let tmp = write_config("[duplication]\ntreshold = 40\nthreshold = 60\n");

    let cfg = Config::load(tmp.path()).unwrap();
    assert_eq!(cfg.duplication.threshold, 60);

    let (cfg, issues) = Config::load_for_scan(tmp.path());
    assert_eq!(cfg.duplication.threshold, 60);
    let messages: Vec<_> = issues.iter().map(|issue| issue.message.as_str()).collect();
    assert_eq!(
        messages,
        vec![
            "sensez.toml:2:1: unknown key `duplication.treshold` ignored (did you mean `threshold`?)"
        ]
    );
}

#[test]
fn unknown_top_level_table_warns_with_suggestion() {
    let tmp = write_config("[deadcode]\nunused_methods = true\n");
    let (_, warnings) = Config::load_with_warnings(tmp.path()).unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0].to_string(),
        "sensez.toml:1:2: unknown key `deadcode` ignored (did you mean `dead_code`?)"
    );
}

#[test]
fn known_config_has_no_warnings() {
    let tmp = write_config(
        "exclude = [\"**/generated/**\"]\n\
         [cache]\nenabled = false\n\
         [duplication]\nthreshold = 55\n\
         [duplication.semantic]\nenabled = false\ncomment_required = true\n\
         [dead_code]\nunused_methods = true\nentrypoints = [\"**/main.py\"]\n\
         [[boundaries.forbidden]]\nfrom = \"a\"\nto = \"b\"\n\
         [action]\ncycles = \"warning\"\n\
         [gate]\nrepeat_limit = 2\n\
         [self_improvement]\nenabled = true\n",
    );

    let (_, warnings) = Config::load_with_warnings(tmp.path()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn unknown_smells_and_action_keys_are_errors_listed_together() {
    let problems = problems(
        "[action]\ncycle = \"info\"\n\
         [smells]\ntypo_toggle = true\n\
         [smells.python]\nmade_up = 1\n\
         [smells.rules.long_function]\nmax_line = 20\n",
    );
    assert_eq!(
        problems,
        vec![
            "sensez.toml:2:1: unknown key `action.cycle` (did you mean `cycles`?)",
            "sensez.toml:4:1: unknown key `smells.typo_toggle`",
            "sensez.toml:6:1: unknown key `smells.python.made_up`",
            "sensez.toml:8:1: unknown key `smells.rules.long_function.max_line` (did you mean `max_lines`?)",
        ]
    );
}

#[test]
fn invalid_enum_is_error_with_location_and_suggestion() {
    let problems = problems("[action]\ncycles = \"must_fx\"\n");
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0].starts_with("sensez.toml:2:10: "),
        "{problems:?}"
    );
    assert!(
        problems[0].ends_with("(did you mean `must_fix`?)"),
        "{problems:?}"
    );
}

#[test]
fn wrong_type_is_error_with_location() {
    let problems = problems("[duplication]\nthreshold = \"fifty\"\n");
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0].starts_with("sensez.toml:2:13: "),
        "{problems:?}"
    );
    assert!(problems[0].contains("invalid type"), "{problems:?}");
}

#[test]
fn syntax_error_is_error_with_location() {
    let problems = problems("[duplication\nthreshold = 5\n");
    assert_eq!(problems.len(), 1);
    assert!(problems[0].starts_with("sensez.toml:1:"), "{problems:?}");
}

#[test]
fn pyproject_problems_point_into_pyproject() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("pyproject.toml"),
        "[project]\nname = \"x\"\n\n[tool.sensez.action]\ncycles = \"critical\"\n",
    )
    .unwrap();
    let errors = Config::load_with_warnings(tmp.path()).unwrap_err();
    assert_eq!(errors.0.len(), 1);
    assert!(
        errors.0[0].to_string().starts_with("pyproject.toml:5:10: "),
        "{errors}"
    );
}

#[test]
fn pyproject_without_tool_sensez_uses_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("pyproject.toml"),
        "[project]\nname = \"x\"\n[tool.ruff]\nline-length = 100\n",
    )
    .unwrap();
    let (cfg, warnings) = Config::load_with_warnings(tmp.path()).unwrap();
    assert_eq!(cfg.duplication.threshold, 50);
    assert!(warnings.is_empty());
}

#[test]
fn old_dead_code_key_names_load_into_the_renamed_fields() {
    let tmp = write_config(
        "[dead_code]\nentrypoints = [\"route\"]\nentry_points = [\"**/scripts/**\"]\n",
    );
    let cfg = Config::load(tmp.path()).unwrap();
    assert_eq!(cfg.dead_code.registration_decorators, ["route"]);
    assert_eq!(cfg.dead_code.entry_globs, ["**/scripts/**"]);
}

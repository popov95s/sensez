//! Semantic duplication config tests.

use super::*;

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

#[test]
fn semantic_score_out_of_range_is_a_config_error() {
    for (knob, message) in [
        (
            "comment_boost_score = 0",
            "comment_boost_score must be at least 50",
        ),
        (
            "min_shape_score = 101",
            "min_shape_score must be at most 100",
        ),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("sensez.toml"),
            format!("[duplication.semantic]\nenabled = true\n{knob}\n"),
        )
        .unwrap();

        let err = Config::load(tmp.path()).unwrap_err();
        assert!(format!("{err:#}").contains(message), "{err:#}");
        let (config, issues) = Config::load_for_scan(tmp.path());
        assert!(!config.duplication.semantic.enabled);
        assert!(
            issues.iter().any(|issue| issue.message.contains(message)),
            "scans must report the invalid knob"
        );
    }
}

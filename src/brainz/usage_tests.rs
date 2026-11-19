use super::*;

#[test]
fn invalid_config_is_listed_in_usage_report() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("sensez.toml"), "exclude = [\"[bad\"]\n").unwrap();

    let report = usage_report(tmp.path());

    let issues = report["config_issues"].as_array().unwrap();
    assert!(issues[0]["message"]
        .as_str()
        .unwrap()
        .contains("invalid glob in exclude"));
}

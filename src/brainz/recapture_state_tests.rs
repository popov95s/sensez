use super::*;

#[test]
fn skips_a_touched_file_when_its_content_is_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("app.py");
    std::fs::write(&file, "value = 1\n").unwrap();
    let baseline = hub::Baseline {
        ts: 0,
        ms: 1,
        threshold: None,
        branch: "main".into(),
        source_state: current(root.path()),
    };

    assert!(!changed(root.path(), &baseline));
    std::fs::write(&file, "value = 2\n").unwrap();
    assert!(changed(root.path(), &baseline));
}

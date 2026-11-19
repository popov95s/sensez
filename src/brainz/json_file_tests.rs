use super::*;
use std::collections::BTreeMap;

#[test]
fn corrupt_file_loads_default_and_keeps_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("triage.json");
    fs::write(&path, "{not json").unwrap();

    let loaded: BTreeMap<String, u64> = load_or_default(&path);

    assert!(loaded.is_empty());
    assert_eq!(
        fs::read_to_string(tmp.path().join("triage.json.bak")).unwrap(),
        "{not json"
    );
}

#[test]
fn missing_file_loads_default_without_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("last-scan.json");

    let loaded: BTreeMap<String, u64> = load_or_default(&path);

    assert!(loaded.is_empty());
    assert!(!tmp.path().join("last-scan.json.bak").exists());
}

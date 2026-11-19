use super::*;

#[test]
fn creates_dir_and_self_ignore() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    std::fs::create_dir_all(&root).unwrap();

    let dir = ensure(&root, Some("local-metrics")).unwrap();
    assert!(dir.ends_with(".sensez/local-metrics") && dir.is_dir());
    let ignore = root.join(".sensez/.gitignore");
    assert_eq!(std::fs::read_to_string(&ignore).unwrap(), "*\n");

    std::fs::write(&ignore, "custom\n").unwrap();
    ensure(&root, None).unwrap();
    assert_eq!(std::fs::read_to_string(&ignore).unwrap(), "custom\n");
}

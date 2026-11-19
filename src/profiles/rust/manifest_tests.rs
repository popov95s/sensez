use super::*;
use std::time::Duration;

#[test]
fn edited_manifest_is_read_again() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("Cargo.toml");
    std::fs::write(&path, "[package]\nname = \"before\"\n").unwrap();
    assert_eq!(
        manifest(tmp.path()).unwrap()["package"]["name"].as_str(),
        Some("before")
    );

    std::fs::write(&path, "[package]\nname = \"after\"\n").unwrap();
    let later = SystemTime::now() + Duration::from_secs(10);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();

    assert_eq!(
        manifest(tmp.path()).unwrap()["package"]["name"].as_str(),
        Some("after")
    );
}

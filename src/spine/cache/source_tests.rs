use super::*;

#[test]
fn loads_source_content_hashes() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("a.py");
    std::fs::write(&file, "x = 1\n").unwrap();
    let first = load(std::slice::from_ref(&file)).unwrap();
    assert_eq!(
        first.sources[0].content_hash,
        fingerprints::hash_bytes(b"x = 1\n")
    );
    std::fs::write(&file, "x = 2\n").unwrap();
    let second = load(&[file]).unwrap();
    assert_ne!(
        first.sources[0].content_hash,
        second.sources[0].content_hash
    );
}

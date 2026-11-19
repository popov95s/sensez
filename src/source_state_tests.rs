use super::*;

fn manifest(entries: &[(&str, u64)]) -> SourceManifest {
    SourceManifest::from_hashes(
        entries
            .iter()
            .map(|(path, hash)| (PathBuf::from(path), *hash)),
    )
}

#[test]
fn classifies_file_changes() {
    let previous = manifest(&[("a.py", 1), ("b.py", 2)]);
    let current = manifest(&[("a.py", 1), ("b.py", 3), ("c.py", 4)]);

    assert_eq!(
        previous.changes(&current),
        ChangeSet {
            added: vec![PathBuf::from("c.py")],
            modified: vec![PathBuf::from("b.py")],
            deleted: Vec::new(),
            unchanged: vec![PathBuf::from("a.py")],
        }
    );
}

#[test]
fn stores_paths_relative_to_the_scan_root() {
    let root = Path::new("/repo");
    let manifest = SourceManifest::from_root_hashes(root, [(root.join("src/lib.rs"), 7)]);

    assert_eq!(manifest.files.get(Path::new("src/lib.rs")), Some(&7));
}

#[test]
fn state_input_change_invalidates_every_file() {
    let previous = SourceState::new(1, 10, Some("main".into()), manifest(&[("a.py", 1)]));
    let current = SourceState::new(1, 11, Some("main".into()), manifest(&[("a.py", 1)]));

    let changes = previous.changes(&current);
    assert_eq!(changes.modified, vec![PathBuf::from("a.py")]);
    assert!(changes.has_changes());
}

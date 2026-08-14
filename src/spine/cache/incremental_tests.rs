use super::*;

fn source(path: PathBuf, bytes: &[u8]) -> SourceFile {
    SourceFile {
        path,
        content_hash: crate::fingerprints::hash_bytes(bytes),
        bytes: bytes.to_vec(),
    }
}

#[test]
fn manifest_detects_branch_style_add_modify_and_delete() {
    let old = PathBuf::from("old.py");
    let same = PathBuf::from("same.py");
    let changed = PathBuf::from("changed.py");
    let state = ParseCacheState {
        manifest: crate::source_state::SourceManifest::from_hashes([
            (old, 1),
            (same.clone(), 2),
            (changed.clone(), 3),
        ]),
        files: HashMap::new(),
    };
    let stats = state.changes(&[
        SourceFile {
            path: same,
            bytes: Vec::new(),
            content_hash: 2,
        },
        SourceFile {
            path: changed,
            bytes: Vec::new(),
            content_hash: 4,
        },
        SourceFile {
            path: PathBuf::from("added.py"),
            bytes: Vec::new(),
            content_hash: 5,
        },
    ]);
    assert_eq!((stats.added, stats.modified, stats.deleted), (1, 1, 1));
    assert_eq!(
        stats.changed_paths,
        vec![
            PathBuf::from("added.py"),
            PathBuf::from("changed.py"),
            PathBuf::from("old.py"),
        ]
    );
}

#[test]
fn memory_state_reuses_walks_after_file_order_changes() {
    let root = tempfile::tempdir().unwrap();
    let sources = vec![
        source(
            root.path().join("first.py"),
            b"def first():\n    return 1\n",
        ),
        source(
            root.path().join("second.py"),
            b"def second():\n    return 2\n",
        ),
    ];
    let (batch, cold) =
        crate::spine::parser::parse_sources_incremental(&sources, &mut ParseCacheState::default());
    assert_eq!(cold.reusable, 0);
    let mut state = ParseCacheState::default();
    state.replace(&sources, &batch.files);

    let reordered = vec![
        source(sources[1].path.clone(), &sources[1].bytes),
        source(sources[0].path.clone(), &sources[0].bytes),
    ];
    let (reused, stats) = crate::spine::parser::parse_sources_incremental(&reordered, &mut state);
    assert_eq!(stats.reusable, 2);
    for (file_id, file) in reused.files.iter().enumerate() {
        assert!(file
            .walked
            .syntax
            .spans
            .iter()
            .all(|span| span.file_id == file_id as u32));
    }
}

#[test]
fn changed_content_never_restores_a_stale_walk() {
    let original = source(PathBuf::from("module.py"), b"def old():\n    return 1\n");
    let (batch, _) = crate::spine::parser::parse_sources_incremental(
        std::slice::from_ref(&original),
        &mut ParseCacheState::default(),
    );
    let mut state = ParseCacheState::default();
    state.replace(std::slice::from_ref(&original), &batch.files);

    let changed = source(original.path, b"def new():\n    return 2\n");
    let (batch, stats) = crate::spine::parser::parse_sources_incremental(&[changed], &mut state);
    assert_eq!((stats.modified, stats.reusable), (1, 0));
    assert!(batch.files[0]
        .walked
        .symbols
        .declared
        .contains(&"new".into()));
    assert!(!batch.files[0]
        .walked
        .symbols
        .declared
        .contains(&"old".into()));
}

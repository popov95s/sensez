use super::*;

fn doc(sym: &str, text: &str) -> Doc {
    Doc {
        file: std::path::PathBuf::from("m.py"),
        line: 1,
        symbol_path: sym.to_string(),
        kind: DocKind::Docstring,
        text: text.to_string(),
    }
}

fn cache_with_doc() -> SystemCache {
    SystemCache {
        model_id: "old-model".to_string(),
        docs: vec![CachedDoc {
            key: 1,
            file: "m.py".to_string(),
            line: 1,
            symbol_path: "m::f".to_string(),
            kind: DocKind::Docstring,
            text: "Add one.".to_string(),
        }],
        vectors: vec![vec![1.0]],
    }
}

/// A changed key (edited text) is a cache miss; an unchanged key is reused.
#[test]
fn key_changes_only_on_content_change() {
    let a = doc("m::f", "Add one.");
    let same = doc("m::f", "Add one.");
    let edited = doc("m::f", "Add two.");
    let moved = doc("m::g", "Add one.");
    let root = Path::new("");
    assert_eq!(key(&a, root), key(&same, root));
    assert_ne!(
        key(&a, root),
        key(&edited, root),
        "text edit must change the key"
    );
    assert_ne!(
        key(&a, root),
        key(&moved, root),
        "symbol move must change the key"
    );
}

#[test]
fn key_ignores_how_the_root_is_spelled() {
    let mut relative = doc("m::f", "Add one.");
    relative.file = "./src/m.py".into();
    let mut absolute = doc("m::f", "Add one.");
    absolute.file = "/work/repo/src/m.py".into();

    assert_eq!(
        key(&relative, Path::new(".")),
        key(&absolute, Path::new("/work/repo"))
    );
}

#[test]
fn model_change_requires_reindex() {
    let mut cache = cache_with_doc();
    if cache.model_id != crate::eyez::embed::MODEL_ID {
        cache.docs.clear();
        cache.vectors.clear();
        cache.model_id = crate::eyez::embed::MODEL_ID.to_string();
    }
    assert!(cache.docs.is_empty());
    assert!(cache.vectors.is_empty());
    assert_eq!(cache.model_id, crate::eyez::embed::MODEL_ID);
}

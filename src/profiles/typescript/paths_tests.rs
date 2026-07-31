use super::paths::ResolutionCache;

#[test]
fn resolution_cache_loads_once_per_root() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("tsconfig.json"),
        r#"{ /* comment */ "compilerOptions": { "paths": {
            "@/*": ["src/*"], "@scope/library": ["packages/library/src/index.ts"],
        }, }, }"#,
    )
    .unwrap();
    let mut cache = ResolutionCache::default();
    assert_eq!(
        cache.aliases(tmp.path()).resolve("@/first"),
        Some("src/first".into())
    );
    assert_eq!(
        cache.aliases(tmp.path()).resolve("@scope/library"),
        Some("packages/library/src".into())
    );
    assert_eq!(cache.load_count(), 1);
}

#[test]
fn resolution_cache_keeps_unavailable_config_result() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = ResolutionCache::default();
    assert_eq!(cache.aliases(tmp.path()).resolve("@/first"), None);
    assert_eq!(cache.aliases(tmp.path()).resolve("@/second"), None);
    assert_eq!(cache.load_count(), 1);
}

#[test]
fn resolution_cache_scopes_aliases_by_root() {
    let tmp = tempfile::tempdir().unwrap();
    let first = tmp.path().join("first");
    let second = tmp.path().join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(
        first.join("tsconfig.json"),
        r#"{ "compilerOptions": { "paths": { "@/*": ["first/*"] } } }"#,
    )
    .unwrap();
    std::fs::write(
        second.join("tsconfig.json"),
        r#"{ "compilerOptions": { "paths": { "@/*": ["second/*"] } } }"#,
    )
    .unwrap();
    let mut cache = ResolutionCache::default();
    assert_eq!(
        cache.aliases(&first).resolve("@/item"),
        Some("first/item".into())
    );
    assert_eq!(
        cache.aliases(&second).resolve("@/item"),
        Some("second/item".into())
    );
    assert_eq!(cache.load_count(), 2);
}

#[test]
fn resolution_cache_keeps_malformed_config_result() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("tsconfig.json"), "not JSONC").unwrap();
    let mut cache = ResolutionCache::default();
    assert_eq!(cache.aliases(tmp.path()).resolve("@/first"), None);
    assert_eq!(cache.aliases(tmp.path()).resolve("@/second"), None);
    assert_eq!(cache.load_count(), 1);
}

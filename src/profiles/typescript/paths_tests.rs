use super::paths::ResolutionCache;
use crate::test_support::write;
use std::path::{Path, PathBuf};

fn targets(cache: &mut ResolutionCache, dir: &Path, specifier: &str) -> Vec<PathBuf> {
    cache
        .aliases(dir)
        .map(|paths| paths.targets(specifier))
        .unwrap_or_default()
}

#[test]
fn resolution_cache_loads_once_per_config() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "tsconfig.json",
        r#"{ /* comment */ "compilerOptions": { "paths": {
            "@/*": ["src/*"], "@scope/library": ["packages/library/src/index.ts"],
        }, }, }"#,
    );
    let mut cache = ResolutionCache::default();
    assert_eq!(
        targets(&mut cache, tmp.path(), "@/first"),
        [tmp.path().join("src/first")]
    );
    assert_eq!(
        targets(&mut cache, tmp.path(), "@scope/library"),
        [tmp.path().join("packages/library/src/index.ts")]
    );
    assert_eq!(cache.load_count(), 1);
}

#[test]
fn resolution_cache_without_config_loads_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = ResolutionCache::default();
    assert!(targets(&mut cache, tmp.path(), "@/first").is_empty());
    assert!(targets(&mut cache, tmp.path(), "@/second").is_empty());
    assert_eq!(cache.load_count(), 0);
}

#[test]
fn resolution_cache_scopes_aliases_by_config() {
    let tmp = tempfile::tempdir().unwrap();
    let first = tmp.path().join("first");
    let second = tmp.path().join("second");
    write(
        &first,
        "tsconfig.json",
        r#"{ "compilerOptions": { "paths": { "@/*": ["first/*"] } } }"#,
    );
    write(
        &second,
        "tsconfig.json",
        r#"{ "compilerOptions": { "paths": { "@/*": ["second/*"] } } }"#,
    );
    let mut cache = ResolutionCache::default();
    assert_eq!(
        targets(&mut cache, &first, "@/item"),
        [first.join("first/item")]
    );
    assert_eq!(
        targets(&mut cache, &second, "@/item"),
        [second.join("second/item")]
    );
    assert_eq!(cache.load_count(), 2);
}

#[test]
fn resolution_cache_keeps_malformed_config_result() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "tsconfig.json", "not JSONC");
    let mut cache = ResolutionCache::default();
    assert!(targets(&mut cache, tmp.path(), "@/first").is_empty());
    assert!(targets(&mut cache, tmp.path(), "@/second").is_empty());
    assert_eq!(cache.load_count(), 1);
}

#[test]
fn aliases_come_from_an_extends_base() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "tsconfig.json",
        r#"{ "extends": "./tsconfig.base.json" }"#,
    );
    write(
        tmp.path(),
        "tsconfig.base.json",
        r##"{ "compilerOptions": { "paths": { "#lib/*": ["src/lib/*"] } } }"##,
    );
    let mut cache = ResolutionCache::default();

    assert_eq!(
        targets(&mut cache, tmp.path(), "#lib/util"),
        [tmp.path().join("src/lib/util")]
    );
}

#[test]
fn base_paths_resolve_against_the_child_base_url() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "app/tsconfig.json",
        r#"{ "extends": "../tsconfig.base.json", "compilerOptions": { "baseUrl": "src" } }"#,
    );
    write(
        tmp.path(),
        "tsconfig.base.json",
        r#"{ "compilerOptions": { "paths": { "@/*": ["*"] } } }"#,
    );
    let mut cache = ResolutionCache::default();

    assert_eq!(
        targets(&mut cache, &tmp.path().join("app"), "@/x"),
        [tmp.path().join("app/src/x")]
    );
}

#[test]
fn nearest_config_above_a_nested_package_applies() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "tsconfig.json",
        r#"{ "compilerOptions": { "paths": { "@web/*": ["packages/web/src/*"] } } }"#,
    );
    write(tmp.path(), "packages/web/package.json", "{}");
    let mut cache = ResolutionCache::default();

    assert_eq!(
        targets(
            &mut cache,
            &tmp.path().join("packages/web/src"),
            "@web/util"
        ),
        [tmp.path().join("packages/web/src/util")]
    );
}

#[test]
fn every_target_of_a_pattern_is_kept_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "tsconfig.json",
        r#"{ "compilerOptions": { "paths": { "@/*": ["src/*", "generated/*"] } } }"#,
    );
    let mut cache = ResolutionCache::default();

    assert_eq!(
        targets(&mut cache, tmp.path(), "@/api"),
        [tmp.path().join("src/api"), tmp.path().join("generated/api")]
    );
}

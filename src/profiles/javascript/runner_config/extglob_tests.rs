use super::super::defaults::{JEST_TEST_MATCH, VITEST_EXCLUDE, VITEST_INCLUDE};
use super::*;

fn matcher(patterns: &[&str]) -> globset::GlobSet {
    let owned: Vec<String> = patterns.iter().map(|pattern| pattern.to_string()).collect();
    super::super::glob_set(&owned).unwrap()
}

#[test]
fn translates_optional_and_one_of_groups() {
    assert_eq!(
        to_globset("**/*.{test,spec}.?(c|m)[jt]s?(x)").unwrap(),
        "**/*.{test,spec}.{,c,m}[jt]s{,x}"
    );
    assert_eq!(to_globset("./src/@(a|b).ts").unwrap(), "src/{a,b}.ts");
    assert_eq!(
        to_globset("**/?(*.)+(spec|test).?([mc])[jt]s?(x)").unwrap(),
        "**/{,*.}{spec,test}.{,[mc]}[jt]s{,x}"
    );
}

#[test]
fn rejects_unsupported_extglobs() {
    assert!(to_globset("**/!(skip).ts").is_err());
    assert!(to_globset("**/*(a|b).ts").is_err());
    assert!(to_globset("{a,?(b|c)}.ts").is_err());
}

#[test]
fn vitest_default_include_matches_runner_semantics() {
    let include = matcher(&VITEST_INCLUDE);
    for path in [
        "a.test.ts",
        "src/a.spec.tsx",
        "src/a.test.mjs",
        "src/a.test.cjs",
        "src/a.spec.jsx",
    ] {
        assert!(include.is_match(path), "{path}");
    }
    for path in [
        "src/__tests__/helper.ts",
        "src/a.test.css",
        "src/a.testkit.ts",
    ] {
        assert!(!include.is_match(path), "{path}");
    }
}

#[test]
fn vitest_default_exclude_matches_runner_semantics() {
    let exclude = matcher(&VITEST_EXCLUDE);
    for path in [
        "node_modules/x/a.test.ts",
        "packages/app/dist/a.test.js",
        ".git/a.test.ts",
        "vitest.config.ts",
    ] {
        assert!(exclude.is_match(path), "{path}");
    }
    assert!(!exclude.is_match("src/a.test.ts"));
}

#[test]
fn jest_default_test_match_keeps_tests_dirs() {
    let include = matcher(&JEST_TEST_MATCH);
    for path in [
        "/repo/src/__tests__/thing.js",
        "/repo/src/a.test.ts",
        "/repo/src/a.spec.mjs",
        "/repo/test.js",
    ] {
        assert!(include.is_match(path), "{path}");
    }
    assert!(!include.is_match("/repo/src/helper.ts"));
}

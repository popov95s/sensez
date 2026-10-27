use super::for_files;
use crate::spine::parser::{parse_file, ParsedFile};
use crate::test_support::write;
use std::path::{Path, PathBuf};

fn parsed(root: &Path, files: &[&str]) -> Vec<ParsedFile> {
    files
        .iter()
        .enumerate()
        .map(|(i, rel)| parse_file(&write(root, rel, "export const x = 1;\n"), i as u32).unwrap())
        .collect()
}

fn names(files: &[ParsedFile], configured: &[PathBuf]) -> Vec<String> {
    for_files(files, configured)
        .into_iter()
        .map(|identity| identity.name)
        .collect()
}

#[test]
fn same_logical_name_in_two_packages_is_named_from_the_enclosing_package() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "package.json", "{}");
    write(tmp.path(), "packages/a/package.json", "{}");
    write(tmp.path(), "packages/b/package.json", "{}");
    let files = parsed(tmp.path(), &["packages/a/x.ts", "packages/b/x.ts"]);

    assert_eq!(names(&files, &[]), ["packages/a/x", "packages/b/x"]);
}

#[test]
fn configured_root_wins_over_an_enclosing_package() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "package.json", "{}");
    let repo = tmp.path().join("repo");
    let files = parsed(tmp.path(), &["repo/a/x.ts", "repo/b/x.ts"]);
    let configured = [repo.join("a"), repo.join("b"), repo];

    assert_eq!(names(&files, &configured), ["a/x", "b/x"]);
}

#[test]
fn without_package_json_names_are_relative_to_the_common_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let files = parsed(tmp.path(), &["a/x.ts", "b/x.ts"]);

    assert_eq!(names(&files, &[]), ["a/x", "b/x"]);
}

#[test]
fn unique_names_keep_their_logical_name() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "a/package.json", "{}");
    let files = parsed(tmp.path(), &["a/x.ts", "a/y.ts"]);

    assert_eq!(names(&files, &[]), ["x", "y"]);
}

use super::*;
use std::fs;

/// Test predicate: keep `.py` files only. Stand-in for what production
/// callers build from `profiles::registry::parse_for_path`.
fn is_python(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "py")
}

#[test]
fn finds_py_files_and_skips_others() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("pkg")).unwrap();
    fs::write(dir.join("a.py"), "x = 1\n").unwrap();
    fs::write(dir.join("pkg/b.py"), "y = 2\n").unwrap();
    fs::write(dir.join("notes.txt"), "ignore me\n").unwrap();

    let found = collect_source_files(&dir, &[], &is_python).unwrap();
    assert!(found.issues.is_empty(), "readable tree skips nothing");
    let names: Vec<_> = found
        .files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(names.contains(&"a.py".to_string()));
    assert!(names.contains(&"b.py".to_string()));
    assert!(!names.iter().any(|n| n.ends_with(".txt")));

    // exclude glob drops matching files from discovery
    let filtered = collect_source_files(&dir, &["**/pkg/**".to_string()], &is_python).unwrap();
    let fnames: Vec<_> = filtered
        .files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(fnames.contains(&"a.py".to_string()));
    assert!(!fnames.contains(&"b.py".to_string()), "pkg/ excluded");
}

#[test]
fn missing_path_errors() {
    assert!(collect_source_files(Path::new("/nonexistent/sensez/xyz"), &[], &is_python).is_err());
}

#[test]
fn walk_errors_become_discover_issues_with_paths() {
    let err = IgnoreError::WithPath {
        path: PathBuf::from("blocked.py"),
        err: Box::new(IgnoreError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "permission denied",
        ))),
    };

    let issue = scan_issue_from_walk_error(&err);
    assert_eq!(issue.stage, ScanStage::Discover);
    assert_eq!(issue.file, Some(PathBuf::from("blocked.py")));
    assert!(issue.message.contains("permission denied"));
}

#[cfg(unix)]
#[test]
fn symlinked_sources_are_included_and_broken_links_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(dir.join("real.py"), "x = 1\n").unwrap();
    std::os::unix::fs::symlink(dir.join("real.py"), dir.join("link.py")).unwrap();
    fs::create_dir(dir.join("pkg")).unwrap();
    fs::write(dir.join("pkg/nested.py"), "y = 2\n").unwrap();
    std::os::unix::fs::symlink(dir.join("pkg"), dir.join("pkg_link")).unwrap();
    std::os::unix::fs::symlink(dir.join("missing.py"), dir.join("broken.py")).unwrap();

    let found = collect_source_files(dir, &[], &is_python).unwrap();
    let names: Vec<_> = found
        .files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(names.contains(&"real.py".to_string()));
    assert!(
        names.contains(&"link.py".to_string()),
        "file symlink must be discovered, got {names:?}"
    );
    // The directory symlink is not followed: `pkg/nested.py` is reachable
    // exactly once through the real directory.
    assert_eq!(
        names.iter().filter(|n| *n == "nested.py").count(),
        1,
        "directory symlinks must not double-count targets"
    );
    assert!(
        !names
            .iter()
            .any(|n| n.ends_with("nested.py") && n.contains("pkg_link")),
        "no path through pkg_link"
    );
    assert_eq!(
        found.issues.len(),
        1,
        "the broken symlink must be reported, not dropped silently"
    );
    assert!(found.issues[0].message.contains("unreadable symlink"));
}

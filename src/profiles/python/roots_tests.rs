use super::*;

#[test]
fn module_name_flat_and_init() {
    let root = Path::new("/proj");
    assert_eq!(
        module_name(Path::new("/proj/pkg/sub/mod.py"), root),
        "pkg.sub.mod"
    );
    assert_eq!(module_name(Path::new("/proj/pkg/__init__.py"), root), "pkg");
    assert_eq!(module_name(Path::new("/proj/top.py"), root), "top");
}

/// A pyproject marker anchors the root, so a namespace package (no
/// `__init__.py`) is still named with its full dotted path.
#[test]
fn marker_root_handles_namespace_packages() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("app/db")).unwrap(); // NO __init__.py anywhere
    fs::write(dir.join("pyproject.toml"), "[project]\nname = \"x\"\n").unwrap();
    let f = dir.join("app/db/models.py");
    fs::write(&f, "X = 1\n").unwrap();

    let root = root_for(&f);
    assert_eq!(root, dir, "root anchored at the marker dir");
    assert_eq!(module_name(&f, &root), "app.db.models");
}

/// src-layout: files under `<marker>/src` are named relative to `src/`.
#[test]
fn marker_root_handles_src_layout() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("src/pkg")).unwrap();
    fs::write(dir.join("pyproject.toml"), "[project]\nname = \"x\"\n").unwrap();
    let f = dir.join("src/pkg/mod.py");
    fs::write(&f, "Y = 1\n").unwrap();

    let root = root_for(&f);
    assert_eq!(root, dir.join("src"), "src-layout root is <marker>/src");
    assert_eq!(module_name(&f, &root), "pkg.mod");
}

/// An installed package under `site-packages` must NOT pick up a surrounding
/// repo's marker — the walk stops at the vendor boundary and falls back to
/// the `__init__` walk.
#[test]
fn marker_walk_stops_at_vendor_boundary() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let sp = dir.join(".venv/lib/site-packages");
    fs::create_dir_all(sp.join("pkg")).unwrap();
    fs::write(dir.join("pyproject.toml"), "[project]\nname = \"repo\"\n").unwrap();
    fs::write(sp.join("pkg/__init__.py"), "").unwrap();
    let f = sp.join("pkg/mod.py");
    fs::write(&f, "Z = 1\n").unwrap();

    let root = root_for(&f);
    assert_eq!(root, sp, "root falls back to site-packages, not the repo");
    assert_eq!(module_name(&f, &root), "pkg.mod");
}

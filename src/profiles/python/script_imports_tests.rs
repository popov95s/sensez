use super::*;
use std::fs;

#[test]
fn script_folders_resolve_sibling_imports() {
    let tmp = tempfile::tempdir().unwrap();
    let scripts = tmp.path().join("tools");
    let package = tmp.path().join("pkg");
    for dir in [&scripts, &package] {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("models.py"), "").unwrap();
    }
    fs::write(package.join("__init__.py"), "").unwrap();

    let script = scripts.join("run.py");
    assert_eq!(
        sibling_module("models", "tools", &script).as_deref(),
        Some("tools.models")
    );
    assert_eq!(sibling_module("json", "tools", &script), None);
    assert_eq!(
        sibling_module("models", "pkg", &package.join("run.py")),
        None
    );
}

//! Script-directory imports: a script's own folder is first on `sys.path`.

use std::path::Path;

/// `target` as a sibling module of an importer outside any package (`from models import X`).
pub fn sibling_module(
    target: &str,
    importer_package: &str,
    importer_file: &Path,
) -> Option<String> {
    if target.starts_with('.') || importer_package.is_empty() {
        return None;
    }
    let dir = importer_file.parent()?;
    let head = target.split('.').next()?;
    let is_sibling = !dir.join("__init__.py").exists() && dir.join(format!("{head}.py")).is_file();
    is_sibling.then(|| format!("{importer_package}.{target}"))
}

#[cfg(test)]
mod tests {
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
}

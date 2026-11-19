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
#[path = "script_imports_tests.rs"]
mod tests;

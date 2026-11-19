//! Resolve a JS/TS import specifier to a module key.

use super::roots;
use crate::spine::ir::ImportContext;
use std::path::{Path, PathBuf};

/// The package (directory key) containing `module_name`.
pub fn containing_package(module_name: &str, is_index: bool) -> String {
    if is_index {
        return module_name.to_string();
    }
    match module_name.rsplit_once('/') {
        Some((pkg, _)) => pkg.to_string(),
        None => String::new(),
    }
}

/// Path key a relative specifier names, matched against parsed files before string keys.
pub fn relative_path_key(import: &ImportContext, importer_file: &Path) -> Option<PathBuf> {
    let target = &import.target_module;
    target.starts_with('.').then(|| {
        let base = importer_file.parent().unwrap_or(Path::new("."));
        roots::path_key(&base.join(target))
    })
}

/// Resolve `import.target_module` to a module key in the JS namespace.
pub fn resolve_target(import: &ImportContext, importer_file: &Path, root: &Path) -> String {
    let target = &import.target_module;
    if !target.starts_with('.') {
        return target.clone(); // bare specifier → external dependency
    }
    let base = importer_file.parent().unwrap_or(Path::new("."));
    let joined = roots::normalize(&base.join(target));
    let rel = joined
        .strip_prefix(roots::normalize(root))
        .unwrap_or(&joined);
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    roots::key_from_parts(parts)
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod tests;

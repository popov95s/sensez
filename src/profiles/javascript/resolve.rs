//! Resolve a JS/TS import specifier to a module key.
//!
//! Relative specifiers (`./foo`, `../bar/baz`) resolve against the importing
//! file's directory on the filesystem, then map to a root-relative path key
//! (extension dropped, `index` collapsed) — matching the keys `roots` assigns
//! to nodes. Bare specifiers (`react`, `@scope/pkg`) are returned unchanged and
//! become external nodes.

use super::roots;
use crate::spine::ir::ImportContext;
use std::path::Path;

/// The package (directory key) containing `module_name`. An index module is its
/// own package; otherwise drop the last path segment.
pub fn containing_package(module_name: &str, is_index: bool) -> String {
    if is_index {
        return module_name.to_string();
    }
    match module_name.rsplit_once('/') {
        Some((pkg, _)) => pkg.to_string(),
        None => String::new(),
    }
}

/// Resolve `import.target_module` to a module key in the JS namespace.
pub fn resolve_target(import: &ImportContext, importer_file: &Path, root: &Path) -> String {
    let target = &import.target_module;
    if !target.starts_with('.') {
        return target.clone(); // bare specifier → external dependency
    }
    let base = importer_file.parent().unwrap_or(Path::new("."));
    let joined = roots::normalize(&base.join(target));
    let rel = joined.strip_prefix(root).unwrap_or(&joined);
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    roots::key_from_parts(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ctx(target: &str) -> ImportContext {
        ImportContext {
            source_module: "x".into(),
            target_module: target.into(),
            imported_symbols: vec![],
            bindings: vec![],
            line: 1,
            column: 1,
            is_inline: false,
            is_module_decl: false,
            enclosing_scope: None,
        }
    }

    #[test]
    fn relative_resolves_to_sibling_key() {
        let root = Path::new("/proj");
        let importer = PathBuf::from("/proj/src/api/handlers.ts");
        assert_eq!(
            resolve_target(&ctx("./util"), &importer, root),
            "src/api/util"
        );
        assert_eq!(
            resolve_target(&ctx("../models"), &importer, root),
            "src/models"
        );
        // `./` directory import collapses to the dir key (its index module).
        assert_eq!(
            resolve_target(&ctx("../db/index"), &importer, root),
            "src/db"
        );
    }

    #[test]
    fn bare_specifier_is_external() {
        let root = Path::new("/proj");
        let importer = PathBuf::from("/proj/src/a.ts");
        assert_eq!(resolve_target(&ctx("react"), &importer, root), "react");
        assert_eq!(
            resolve_target(&ctx("@scope/pkg"), &importer, root),
            "@scope/pkg"
        );
    }

    #[test]
    fn containing_package_rules() {
        assert_eq!(containing_package("src/api/util", false), "src/api");
        assert_eq!(containing_package("src/api", true), "src/api");
        assert_eq!(containing_package("main", false), "");
    }
}

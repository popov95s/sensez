//! Resolve an import's target to a known module name.
//!
//! Absolute imports look up the dotted target directly. Relative imports
//! (`from . import x`, `from ..pkg import y`) are resolved against the
//! importing module's package per Python's leading-dot semantics.

use crate::spine::ir::ImportContext;

/// The package containing `module_name`.
///
/// A regular module `pkg.sub.mod` has package `pkg.sub`. A package's
/// `__init__` (module name `pkg.sub`) is its own package, so callers pass
/// `is_package = true` for `__init__.py` files.
pub fn containing_package(module_name: &str, is_package: bool) -> String {
    if is_package {
        return module_name.to_string();
    }
    match module_name.rsplit_once('.') {
        Some((pkg, _)) => pkg.to_string(),
        None => String::new(),
    }
}

/// Resolve the target module name for an import edge.
///
/// Returns the absolute dotted module name the edge points at. Whether that
/// name corresponds to a known node is decided by the caller (registry lookup).
pub fn resolve_target(import: &ImportContext, importer_package: &str) -> String {
    let target = &import.target_module;
    let dots = target.chars().take_while(|c| *c == '.').count();
    if dots == 0 {
        return target.clone();
    }
    resolve_relative(target, dots, importer_package)
}

fn resolve_relative(target: &str, dots: usize, importer_package: &str) -> String {
    let remainder = &target[dots..];
    let mut parts: Vec<&str> = if importer_package.is_empty() {
        Vec::new()
    } else {
        importer_package.split('.').collect()
    };
    // One dot == current package; each extra dot ascends one level.
    for _ in 0..dots.saturating_sub(1) {
        parts.pop();
    }
    if !remainder.is_empty() {
        parts.extend(remainder.split('.'));
    }
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(target: &str) -> ImportContext {
        ImportContext {
            source_module: "x".into(),
            target_module: target.into(),
            imported_symbols: vec![],
            bindings: vec![],
            binding_phases: vec![],
            line: 1,
            column: 1,
            phase: crate::spine::ir::ImportPhase::Runtime,
            is_inline: false,
            is_module_decl: false,
            enclosing_scope: None,
        }
    }

    #[test]
    fn absolute_passthrough() {
        assert_eq!(resolve_target(&ctx("a.b.c"), "pkg.sub"), "a.b.c");
    }

    #[test]
    fn relative_levels() {
        assert_eq!(resolve_target(&ctx("."), "pkg.sub"), "pkg.sub");
        assert_eq!(resolve_target(&ctx(".mod"), "pkg.sub"), "pkg.sub.mod");
        assert_eq!(resolve_target(&ctx("..other"), "pkg.sub"), "pkg.other");
        assert_eq!(resolve_target(&ctx(".."), "pkg.sub"), "pkg");
    }

    #[test]
    fn containing_package_rules() {
        assert_eq!(containing_package("pkg.sub.mod", false), "pkg.sub");
        assert_eq!(containing_package("pkg.sub", true), "pkg.sub");
        assert_eq!(containing_package("top", false), "");
    }
}

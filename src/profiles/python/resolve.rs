//! Resolve an import's target to a known module name.

use crate::spine::ir::ImportContext;

/// The package containing `module_name`.
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
    for _ in 0..dots.saturating_sub(1) {
        parts.pop();
    }
    if !remainder.is_empty() {
        parts.extend(remainder.split('.'));
    }
    parts.join(".")
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod tests;

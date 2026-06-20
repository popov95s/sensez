//! Python language profile.

mod classunit;
mod conditionals;
mod deadcode;
mod imports;
mod lexeme;
mod obsession;
mod resolve;
mod roots;
mod scope;
mod symbols;
mod tokens;
mod traversal;
mod typehints;
pub(crate) mod typevocab;
mod units;

use crate::profiles::profile_macro::lang_profile;
use crate::profiles::Language;
use std::path::Path;

lang_profile! {
    /// The Python language profile (zero-sized).
    pub struct PythonProfile {
        info: INFO,
        language: Language::Python,
        extensions: &["py", "pyi"],
        grammar: tree_sitter_python::LANGUAGE,
        walk: traversal::walk,
        root_for: roots::root_for,
        module_name: roots::module_name,
        is_package_index: roots::is_package_index,
        containing_package: resolve::containing_package,
        // Dotted-package semantics: resolution needs only the importer's package.
        resolve_target: |import, pkg, _file, _root| resolve::resolve_target(import, pkg),
        // `from pkg import name` — `pkg.name` may be a submodule.
        submodule_candidate: |target, symbol| Some(format!("{target}.{symbol}")),
        decorators: true,
        classify_decorator: deadcode::classify,
        is_conventionally_private: deadcode::is_conventionally_private,
        is_entry_file_stem: deadcode::is_entry_file_stem,
        dead_code_defaults: deadcode::defaults,
        entry_modules: entry_modules_from_pyproject,
        // `__init__` ↔ submodule mutual imports are real load-time hazards.
        is_containment: |_importer, _target| false,
    }
}

/// Collect module names targeted by `pyproject.toml` entry points
/// (`[project.scripts]`, `[project.gui-scripts]`, `[project.entry-points.*]`).
/// A target `"pkg.mod:attr"` yields the module `"pkg.mod"`. Missing/invalid
/// pyproject is ignored — entry-point derivation is best-effort.
fn entry_modules_from_pyproject(project_root: &Path) -> Vec<String> {
    let text = match std::fs::read_to_string(project_root.join("pyproject.toml")) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let value: toml::Value = match toml::from_str(&text) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let project = match value.get("project") {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut modules = Vec::new();
    let mut push = |target: &str| {
        let module = target.split(':').next().unwrap_or(target).trim();
        if !module.is_empty() {
            modules.push(module.to_string());
        }
    };
    for key in ["scripts", "gui-scripts"] {
        if let Some(table) = project.get(key).and_then(toml::Value::as_table) {
            table
                .values()
                .filter_map(toml::Value::as_str)
                .for_each(&mut push);
        }
    }
    if let Some(groups) = project.get("entry-points").and_then(toml::Value::as_table) {
        for group in groups.values().filter_map(toml::Value::as_table) {
            group
                .values()
                .filter_map(toml::Value::as_str)
                .for_each(&mut push);
        }
    }
    modules
}

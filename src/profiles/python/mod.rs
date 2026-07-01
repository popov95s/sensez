//! Python language profile.

mod classunit;
mod conditionals;
mod deadcode;
mod generated;
mod imports;
mod lexeme;
mod obsession;
mod performance;
mod resolve;
mod roots;
mod scope;
mod symbols;
mod tokens;
mod traversal;
mod typehints;
pub(crate) mod typevocab;
mod units;

use crate::profiles::{
    DeadCodeProfile, Language, LanguageInfo, ModuleProfile, ParseProfile, PerformanceProfile,
};
use crate::spine::ir::{ClassProperty, ClassUnit, ImportContext, Walked};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The Python language profile (zero-sized).
pub struct PythonProfile;

static PYTHON_INFO: LanguageInfo = LanguageInfo {
    language: Language::Python,
    extensions: &["py", "pyi"],
};

impl ParseProfile for PythonProfile {
    fn info(&self) -> &'static LanguageInfo {
        &PYTHON_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn walk(&self, root: tree_sitter::Node, src: &[u8], file_id: u32, module_name: &str) -> Walked {
        traversal::walk(root, src, file_id, module_name)
    }

    fn is_generated_or_data_source(&self, path: &Path) -> bool {
        generated::is_generated_or_data_source(path)
    }
}

impl ModuleProfile for PythonProfile {
    fn root_for(&self, file: &Path) -> PathBuf {
        roots::root_for(file)
    }

    fn module_name(&self, file: &Path, root: &Path) -> String {
        roots::module_name(file, root)
    }

    fn is_package_index(&self, file: &Path) -> bool {
        roots::is_package_index(file)
    }

    fn containing_package(&self, module_name: &str, is_index: bool) -> String {
        resolve::containing_package(module_name, is_index)
    }

    fn resolve_target(
        &self,
        import: &ImportContext,
        importer_package: &str,
        _importer_file: &Path,
        _root: &Path,
    ) -> String {
        // Dotted-package semantics: resolution needs only the importer's package.
        resolve::resolve_target(import, importer_package)
    }

    fn submodule_candidate(&self, target: &str, symbol: &str) -> Option<String> {
        // `from pkg import name` — `pkg.name` may be a submodule.
        Some(format!("{target}.{symbol}"))
    }

    fn is_containment(&self, _importer: &str, _target: &str) -> bool {
        // `__init__` ↔ submodule mutual imports are real load-time hazards.
        false
    }
}

impl DeadCodeProfile for PythonProfile {
    fn classify_decorator(
        &self,
        paths: Option<&Vec<String>>,
        user_entrypoints: &HashSet<String>,
    ) -> crate::profiles::DecoratorClass {
        deadcode::classify(paths, user_entrypoints)
    }

    fn is_conventionally_private(&self, symbol: &str) -> bool {
        deadcode::is_conventionally_private(symbol)
    }

    fn is_entry_file_stem(&self, stem: &str) -> bool {
        deadcode::is_entry_file_stem(stem)
    }

    fn manages_class_properties(
        &self,
        class: &ClassUnit,
        decorators: Option<&Vec<String>>,
    ) -> bool {
        deadcode::manages_class_properties(class, decorators)
    }

    fn manages_property(&self, property: &ClassProperty) -> bool {
        deadcode::manages_property(property)
    }

    fn requires_property_usage_evidence(&self, class: &ClassUnit) -> bool {
        deadcode::requires_property_usage_evidence(class)
    }

    fn dead_code_defaults(&self) -> crate::profiles::DeadCodeDefaults {
        deadcode::defaults()
    }

    fn entry_modules(&self, project_root: &Path) -> Vec<String> {
        entry_modules_from_pyproject(project_root)
    }
}

impl PerformanceProfile for PythonProfile {
    fn is_expensive_loop_call(&self, method: &str) -> bool {
        performance::EXPENSIVE_LOOP_METHODS.contains(&method)
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

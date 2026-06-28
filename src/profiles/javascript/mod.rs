//! JavaScript language profile. TypeScript reuses these helpers with a
//! different grammar.

pub(crate) mod classunit;
pub(crate) mod conditionals;
pub(crate) mod deadcode;
pub(crate) mod imports;
pub(crate) mod lexeme;
pub(crate) mod obsession;
pub(crate) mod performance;
pub(crate) mod resolve;
pub(crate) mod roots;
pub(crate) mod scope;
pub(crate) mod symbols;
pub(crate) mod tokens;
pub(crate) mod traversal;
pub(crate) mod typehints;
pub(crate) mod typevocab;
pub(crate) mod units;

#[cfg(test)]
mod tests;

use crate::profiles::{
    DeadCodeProfile, Language, LanguageInfo, ModuleProfile, ParseProfile, PerformanceProfile,
};
use crate::spine::ir::{ImportContext, Walked};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The JavaScript language profile (zero-sized).
pub struct JsProfile;

static JS_INFO: LanguageInfo = LanguageInfo {
    language: Language::JavaScript,
    extensions: &["js", "jsx", "mjs", "cjs"],
};

impl ParseProfile for JsProfile {
    fn info(&self) -> &'static LanguageInfo {
        &JS_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn walk(&self, root: tree_sitter::Node, src: &[u8], file_id: u32, module_name: &str) -> Walked {
        traversal::walk(root, src, file_id, module_name)
    }
}

impl ModuleProfile for JsProfile {
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
        _importer_package: &str,
        file: &Path,
        root: &Path,
    ) -> String {
        // `./foo` resolves against the importing file's directory on disk.
        resolve::resolve_target(import, file, root)
    }

    fn submodule_candidate(&self, _target: &str, _symbol: &str) -> Option<String> {
        // JS named imports are symbols, never submodules.
        None
    }

    fn is_containment(&self, _importer: &str, _target: &str) -> bool {
        false
    }
}

impl DeadCodeProfile for JsProfile {
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

    fn dead_code_defaults(&self) -> crate::profiles::DeadCodeDefaults {
        deadcode::defaults()
    }

    fn entry_modules(&self, _project_root: &Path) -> Vec<String> {
        // package.json bin/main derivation: deferred milestone.
        Vec::new()
    }
}

impl PerformanceProfile for JsProfile {
    fn is_expensive_loop_call(&self, method: &str) -> bool {
        performance::EXPENSIVE_LOOP_METHODS.contains(&method)
    }
}

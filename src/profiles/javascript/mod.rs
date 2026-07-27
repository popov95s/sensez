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
pub(crate) mod risk_facts;
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
    DeadCodeProfile, Language, LanguageInfo, ModuleLayout, ModuleProfile, ParseProfile,
    PerformanceProfile, TypeVocabularyProfile,
};
use crate::spine::ir::{ImportContext, Walked};
use std::collections::HashSet;
use std::path::Path;

/// The JavaScript language profile (zero-sized).
pub struct JsProfile;

static JS_INFO: LanguageInfo = LanguageInfo {
    language: Language::JavaScript,
    extensions: &["js", "jsx", "mjs", "cjs"],
};
static JS_MODULES: ModuleLayout = ModuleLayout::new(
    roots::root_for,
    roots::module_name,
    roots::is_package_index,
    resolve::containing_package,
);

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
    fn module_layout(&self) -> ModuleLayout {
        JS_MODULES
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
    fn receiver_root<'a>(&self, receiver: &'a str) -> &'a str {
        performance::receiver_root(receiver)
    }

    fn is_mutating_call(&self, method: &str) -> bool {
        performance::is_mutating_call(method)
    }

    fn is_bounded_loop(&self, subject: &str) -> bool {
        performance::is_bounded_loop(subject)
    }

    fn is_external_loop_call(
        &self,
        method: &str,
        receiver: &str,
        receiver_type: Option<&str>,
        loops: &[crate::spine::ir::PerfLine],
    ) -> bool {
        performance::is_external_loop_call(method, receiver, receiver_type, loops)
    }
}

impl TypeVocabularyProfile for JsProfile {
    fn loose_kind(&self, annotation: &str) -> Option<crate::profiles::typevocab::LooseTypeKind> {
        typevocab::loose_kind(annotation)
    }

    fn is_bool(&self, annotation: &str) -> bool {
        typevocab::is_bool(annotation)
    }

    fn is_dictish(&self, annotation: &str) -> bool {
        typevocab::is_dictish(annotation)
    }

    fn has_domain_model(&self, annotation: &str) -> bool {
        typevocab::has_domain_model(annotation)
    }

    fn is_primitive_scalar_alias(&self, annotation: &str) -> bool {
        typevocab::is_primitive_scalar_alias(annotation)
    }
}

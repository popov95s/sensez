//! Scope matches the JS/TS profiles: structural pillars (duplication, cycles,
//! boundaries), graph-based dead code for `pub` items (rustc's `dead_code` lint
//! already owns private ones), and Rust-native unit extraction for smells that
//! complement Clippy instead of re-reporting its local style lints.

pub(crate) mod deadcode;
pub(crate) mod imports;
pub(crate) mod lexeme;
pub(crate) mod performance;
pub(crate) mod resolve;
pub(crate) mod roots;
pub(crate) mod scope;
pub(crate) mod symbols;
pub(crate) mod tokens;
pub(crate) mod traversal;
pub(crate) mod typevocab;
mod unit_helpers;
pub(crate) mod units;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod units_tests;

use crate::profiles::{
    DeadCodeProfile, Language, LanguageInfo, ModuleLayout, ModuleProfile, ParseProfile,
    PerformanceProfile, TypeVocabularyProfile,
};
use crate::spine::ir::{ImportContext, Walked};
use std::collections::HashSet;
use std::path::Path;

/// The Rust language profile (zero-sized).
pub struct RustProfile;

static RUST_INFO: LanguageInfo = LanguageInfo {
    language: Language::Rust,
    extensions: &["rs"],
};
static RUST_MODULES: ModuleLayout = ModuleLayout::new(
    roots::root_for,
    roots::module_name,
    roots::is_package_index,
    resolve::containing_package,
);

impl ParseProfile for RustProfile {
    fn info(&self) -> &'static LanguageInfo {
        &RUST_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn walk(&self, root: tree_sitter::Node, src: &[u8], file_id: u32, module_name: &str) -> Walked {
        traversal::walk(root, src, file_id, module_name)
    }
}

impl ModuleProfile for RustProfile {
    fn module_layout(&self) -> ModuleLayout {
        RUST_MODULES
    }

    fn resolve_target(
        &self,
        import: &ImportContext,
        _importer_package: &str,
        file: &Path,
        root: &Path,
    ) -> String {
        // `crate::`/`self::`/`super::`/package-name paths resolve on disk.
        resolve::resolve_target(import, file, root)
    }

    fn submodule_candidate(&self, target: &str, symbol: &str) -> Option<String> {
        // `use crate::noze::smells` — the last segment may be a submodule.
        Some(format!("{target}/{symbol}"))
    }

    fn is_containment(&self, importer: &str, target: &str) -> bool {
        // An edge into the importer's own subtree (`use self::builder::build`,
        // façade re-exports) is containment, like `mod builder;` itself.
        target
            .strip_prefix(importer)
            .is_some_and(|rest| rest.starts_with('/'))
    }
}

impl DeadCodeProfile for RustProfile {
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

    fn entry_modules(&self, project_root: &Path) -> Vec<String> {
        deadcode::entry_modules(project_root)
    }
}

impl PerformanceProfile for RustProfile {
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

impl TypeVocabularyProfile for RustProfile {
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

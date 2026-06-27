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
    DeadCodeProfile, Language, LanguageInfo, ModuleProfile, ParseProfile, PerformanceProfile,
};
use crate::spine::ir::{ImportContext, Walked};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The Rust language profile (zero-sized).
pub struct RustProfile;

static RUST_INFO: LanguageInfo = LanguageInfo {
    language: Language::Rust,
    extensions: &["rs"],
};

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
    fn is_expensive_loop_call(&self, method: &str) -> bool {
        performance::EXPENSIVE_LOOP_METHODS.contains(&method)
    }

    fn is_external_get_receiver(&self, base: &str) -> bool {
        performance::EXTERNAL_GET_RECEIVERS.contains(&base)
    }
}

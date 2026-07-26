//! TypeScript language profiles. TS and TSX are distinct tree-sitter grammars
//! selected by extension, so each is its own zero-sized profile — both report
//! [`Language::TypeScript`] and reuse every JavaScript helper (the grammars
//! share node-kind names; TS-only kinds like `interface_declaration` simply map
//! to no structural token). TS decorators are a deferred enhancement.

use crate::profiles::javascript::{deadcode, performance, resolve, roots, traversal};
use crate::profiles::{
    DeadCodeProfile, Language, LanguageInfo, ModuleLayout, ModuleProfile, ParseProfile,
    PerformanceProfile,
};
use crate::spine::ir::{ImportContext, Walked};
use std::collections::HashSet;
use std::path::Path;

mod paths;

static TS_INFO: LanguageInfo = LanguageInfo {
    language: Language::TypeScript,
    extensions: &["ts"],
};

static TSX_INFO: LanguageInfo = LanguageInfo {
    language: Language::TypeScript,
    extensions: &["tsx"],
};
static TYPESCRIPT_MODULES: ModuleLayout = ModuleLayout::new(
    roots::root_for,
    roots::module_name,
    roots::is_package_index,
    resolve::containing_package,
);

/// The TypeScript language profile (zero-sized).
pub struct TsProfile;

impl ParseProfile for TsProfile {
    fn info(&self) -> &'static LanguageInfo {
        &TS_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn walk(&self, root: tree_sitter::Node, src: &[u8], file_id: u32, module_name: &str) -> Walked {
        traversal::walk(root, src, file_id, module_name)
    }
}

/// The TSX language profile (zero-sized).
pub struct TsxProfile;

impl ParseProfile for TsxProfile {
    fn info(&self) -> &'static LanguageInfo {
        &TSX_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    }

    fn walk(&self, root: tree_sitter::Node, src: &[u8], file_id: u32, module_name: &str) -> Walked {
        traversal::walk(root, src, file_id, module_name)
    }
}

// TS and TSX share everything except the language info and the underlying
// tree-sitter grammar, so the rest of the trait impls are identical and
// delegated to a single generic helper. The macro equivalent used to
// duplicate this; the duplication is gone.
macro_rules! impl_ts_traits {
    ($name:ident) => {
        impl ModuleProfile for $name {
            fn module_layout(&self) -> ModuleLayout {
                TYPESCRIPT_MODULES
            }

            fn resolve_target(
                &self,
                import: &ImportContext,
                _importer_package: &str,
                file: &Path,
                root: &Path,
            ) -> String {
                paths::resolve_target(import, file, root)
            }

            fn disambiguated_module_name(
                &self,
                file: &Path,
                workspace_root: &Path,
                _base: &str,
            ) -> String {
                roots::module_name(file, workspace_root)
            }

            fn submodule_candidate(&self, _target: &str, _symbol: &str) -> Option<String> {
                None
            }

            fn is_containment(&self, _importer: &str, _target: &str) -> bool {
                false
            }
        }

        impl DeadCodeProfile for $name {
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
                deadcode::typescript_defaults()
            }

            fn entry_modules(&self, _project_root: &Path) -> Vec<String> {
                Vec::new()
            }
        }

        impl PerformanceProfile for $name {
            fn is_expensive_loop_call(&self, method: &str) -> bool {
                performance::EXPENSIVE_LOOP_METHODS.contains(&method)
            }
        }
    };
}

impl_ts_traits!(TsProfile);
impl_ts_traits!(TsxProfile);

#[cfg(test)]
mod tests;

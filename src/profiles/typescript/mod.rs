//! TypeScript language profiles. TS and TSX are distinct tree-sitter grammars
//! selected by extension, so each is its own zero-sized profile — both report
//! [`Language::TypeScript`] and reuse every JavaScript helper (the grammars
//! share node-kind names; TS-only kinds like `interface_declaration` simply map
//! to no structural token). TS decorators are a deferred enhancement.

use crate::profiles::javascript::{
    deadcode, declaration, generated, manifest, performance, resolve, roots, test_support,
    traversal, typevocab, wording,
};
use crate::profiles::{
    DeadCodeProfile, ImportResolver, Language, LanguageInfo, ModuleProfile, ParseProfile,
    PerformanceProfile, TypeVocabularyProfile,
};
use crate::spine::ir::{MutatorFamily, Walked};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[cfg(test)]
#[path = "dead_code_reach_tests.rs"]
mod dead_code_reach_tests;
#[cfg(test)]
#[path = "dead_code_tests.rs"]
mod dead_code_tests;
#[cfg(test)]
#[path = "duplication_tests.rs"]
mod duplication_tests;
mod edge_phase;
mod paths;
#[cfg(test)]
#[path = "paths_tests.rs"]
mod paths_tests;
mod tsconfig;
mod type_imports;
#[cfg(test)]
#[path = "type_imports_tests.rs"]
mod type_imports_tests;

#[cfg(test)]
#[path = "default_export_tests.rs"]
mod default_export_tests;

#[cfg(test)]
#[path = "wording_tests.rs"]
mod wording_tests;

#[cfg(test)]
#[path = "mutation_tests.rs"]
mod mutation_tests;

#[cfg(test)]
#[path = "typing_tests.rs"]
mod typing_tests;

#[cfg(test)]
#[path = "nested_function_tests.rs"]
mod nested_function_tests;

#[cfg(test)]
#[path = "tuple_tests.rs"]
mod tuple_tests;

#[cfg(test)]
#[path = "arrow_name_tests.rs"]
mod arrow_name_tests;

static TS_INFO: LanguageInfo = LanguageInfo {
    language: Language::TypeScript,
    extensions: &["ts", "mts", "cts"],
    test_support: test_support::TEST_SUPPORT,
    wording: &wording::WORDING,
};

static TSX_INFO: LanguageInfo = LanguageInfo {
    language: Language::TypeScript,
    extensions: &["tsx"],
    test_support: test_support::TEST_SUPPORT,
    wording: &wording::WORDING,
};
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

    fn is_declaration_only(&self, path: &Path) -> bool {
        declaration::is_declaration_file(path)
    }

    fn is_generated(&self, path: &Path, src: &[u8]) -> bool {
        generated::is_generated(path, src)
    }

    fn clone_family(&self) -> Language {
        Language::JavaScript
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

    fn is_generated(&self, path: &Path, src: &[u8]) -> bool {
        generated::is_generated(path, src)
    }

    fn clone_family(&self) -> Language {
        Language::JavaScript
    }
}

// TS and TSX share everything except the language info and the underlying
// tree-sitter grammar, so the rest of the trait impls are identical and
// delegated to a single generic helper. The macro equivalent used to
// duplicate this; the duplication is gone.
macro_rules! impl_ts_traits {
    ($name:ident) => {
        impl ModuleProfile for $name {
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

            fn import_resolver(&self) -> Box<dyn ImportResolver> {
                Box::new(paths::ResolutionCache::default())
            }

            fn disambiguated_module_name(
                &self,
                file: &Path,
                workspace_root: &Path,
                _base: &str,
            ) -> String {
                roots::module_name(file, workspace_root)
            }

            fn workspace_root(&self, common: &Path) -> Option<PathBuf> {
                roots::workspace_root(common)
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

            fn manifest_entry_files(&self, files: &[&Path]) -> Vec<PathBuf> {
                manifest::entry_files(files)
            }
        }

        impl PerformanceProfile for $name {
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

        impl TypeVocabularyProfile for $name {
            fn loose_kind(
                &self,
                annotation: &str,
            ) -> Option<crate::profiles::typevocab::LooseTypeKind> {
                typevocab::loose_kind(annotation)
            }

            fn is_bool(&self, annotation: &str) -> bool {
                typevocab::is_bool(annotation)
            }

            fn is_dictish(&self, annotation: &str) -> bool {
                typevocab::is_dictish(annotation)
            }

            fn is_open_map(&self, annotation: &str) -> bool {
                typevocab::is_open_map(annotation)
            }

            fn is_nominal_type(&self, annotation: &str) -> bool {
                typevocab::is_nominal_type(annotation)
            }

            fn has_domain_model(&self, annotation: &str) -> bool {
                typevocab::has_domain_model(annotation)
            }

            fn is_primitive_scalar_alias(&self, annotation: &str) -> bool {
                typevocab::is_primitive_scalar_alias(annotation)
            }

            fn guarded_param<'a>(&self, return_annotation: &'a str) -> Option<&'a str> {
                typevocab::guarded_param(return_annotation)
            }

            fn is_literal_union(&self, annotation: &str) -> bool {
                typevocab::is_literal_union(annotation)
            }

            fn is_mutable_collection(&self, annotation: &str, family: MutatorFamily) -> bool {
                typevocab::is_mutable_collection(annotation, family)
            }

            fn is_void(&self, annotation: &str) -> bool {
                typevocab::is_void(annotation)
            }
        }
    };
}

impl_ts_traits!(TsProfile);
impl_ts_traits!(TsxProfile);

#[cfg(test)]
mod tests;

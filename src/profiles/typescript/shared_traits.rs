use super::profile::{TsProfile, TsxProfile};
use crate::profiles::dependencies::{DeclaredPackages, KnownUtility};
use crate::profiles::javascript::{
    deadcode, manifest, performance, resolve, roots, typevocab, utilities,
};
use crate::profiles::{
    DeadCodeProfile, DependencyProfile, EntryCandidate, ImportResolver, ModuleProfile,
    PerformanceProfile, TypeVocabularyProfile,
};
use crate::spine::ir::MutatorFamily;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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
                Box::new(super::paths::ResolutionCache::default())
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

            fn path_key(&self, file: &Path) -> Option<PathBuf> {
                Some(roots::path_key(file))
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

            fn manifest_entry_files(&self, files: &[EntryCandidate]) -> Vec<PathBuf> {
                manifest::entry_files(files)
            }
        }

        impl DependencyProfile for $name {
            fn known_utilities(&self) -> &'static [KnownUtility] {
                utilities::UTILITIES
            }

            fn declared_packages(&self, dir: &Path) -> DeclaredPackages {
                manifest::declared_packages(dir)
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

            fn admits_missing(&self, annotation: &str) -> bool {
                typevocab::admits_missing(annotation)
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

use crate::profiles::runner_config::RunnerKind;
use crate::profiles::{
    DeadCodeProfile, ImportResolver, LanguageInfo, ModuleProfile, ParseProfile, PerformanceProfile,
    TypeVocabularyProfile,
};
use crate::spine::ir::{
    ClassProperty, ClassUnit, FileId, ImportContext, Language, MutatorFamily, Walked,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The Python language profile.
pub struct PythonProfile;

static PYTHON_INFO: LanguageInfo = LanguageInfo {
    language: Language::Python,
    extensions: &["py", "pyi"],
    test_support: &[],
    migration_layouts: &["**/alembic/**/*.py", "**/versions/**/*.py"],
    test_runners: &[RunnerKind::Pytest],
    default_off_smells: &[],
    #[cfg(feature = "lsp")]
    project_files: &["pyproject.toml"],
    import_words: &["import"],
    wording: &super::wording::WORDING,
};

impl ParseProfile for PythonProfile {
    fn info(&self) -> &'static LanguageInfo {
        &PYTHON_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn walk(
        &self,
        root: tree_sitter::Node,
        src: &[u8],
        file_id: FileId,
        module_name: &str,
    ) -> Walked {
        super::traversal::walk(root, src, file_id, module_name)
    }

    fn is_generated_or_data_source(&self, path: &Path) -> bool {
        super::generated::is_generated_or_data_source(path)
    }

    fn is_declaration_only(&self, path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "pyi")
    }
}

impl ModuleProfile for PythonProfile {
    fn root_for(&self, file: &Path) -> PathBuf {
        super::roots::root_for(file)
    }

    fn module_name(&self, file: &Path, root: &Path) -> String {
        super::roots::module_name(file, root)
    }

    fn is_package_index(&self, file: &Path) -> bool {
        super::roots::is_package_index(file)
    }

    fn containing_package(&self, module_name: &str, is_index: bool) -> String {
        super::resolve::containing_package(module_name, is_index)
    }

    fn import_resolver(&self) -> Box<dyn ImportResolver> {
        Box::new(PythonProfile)
    }

    fn submodule_candidate(&self, target: &str, symbol: &str) -> Option<String> {
        Some(format!("{target}.{symbol}"))
    }

    fn is_containment(&self, _importer: &str, _target: &str) -> bool {
        false
    }
}

impl ImportResolver for PythonProfile {
    fn resolve_target(
        &mut self,
        import: &ImportContext,
        importer_package: &str,
        importer_file: &Path,
        _root: &Path,
    ) -> String {
        super::script_imports::sibling_module(
            &import.target_module,
            importer_package,
            importer_file,
        )
        .unwrap_or_else(|| super::resolve::resolve_target(import, importer_package))
    }
}

impl DeadCodeProfile for PythonProfile {
    fn classify_decorator(
        &self,
        paths: Option<&Vec<String>>,
        user_entrypoints: &HashSet<String>,
    ) -> crate::profiles::DecoratorClass {
        super::deadcode::classify(paths, user_entrypoints)
    }

    fn is_conventionally_private(&self, symbol: &str) -> bool {
        super::deadcode::is_conventionally_private(symbol)
    }

    fn is_entry_file_stem(&self, stem: &str) -> bool {
        super::deadcode::is_entry_file_stem(stem)
    }

    fn manages_class_properties(
        &self,
        class: &ClassUnit,
        decorators: Option<&Vec<String>>,
    ) -> bool {
        super::deadcode::manages_class_properties(class, decorators)
    }

    fn manages_property(&self, property: &ClassProperty) -> bool {
        super::deadcode::manages_property(property)
    }

    fn requires_property_usage_evidence(&self, class: &ClassUnit) -> bool {
        super::deadcode::requires_property_usage_evidence(class)
    }

    fn dead_code_defaults(&self) -> crate::profiles::DeadCodeDefaults {
        super::deadcode::defaults()
    }

    fn entry_modules(&self, project_root: &Path) -> Vec<String> {
        super::pyproject_entries::entry_modules_from_pyproject(project_root)
    }
}

impl PerformanceProfile for PythonProfile {
    fn receiver_root<'a>(&self, receiver: &'a str) -> &'a str {
        super::performance::receiver_root(receiver)
    }

    fn is_mutating_call(&self, method: &str) -> bool {
        super::performance::is_mutating_call(method)
    }

    fn is_bounded_loop(&self, subject: &str) -> bool {
        super::performance::is_bounded_loop(subject)
    }

    fn is_external_loop_call(
        &self,
        method: &str,
        receiver: &str,
        receiver_type: Option<&str>,
        loops: &[crate::spine::ir::PerfLine],
    ) -> bool {
        super::performance::is_external_loop_call(method, receiver, receiver_type, loops)
    }
}

impl crate::profiles::DependencyProfile for PythonProfile {
    fn known_utilities(&self) -> &'static [crate::profiles::dependencies::KnownUtility] {
        super::utilities::UTILITIES
    }

    fn declared_packages(&self, dir: &Path) -> crate::profiles::dependencies::DeclaredPackages {
        super::manifest::declared_packages(dir)
    }
}

impl TypeVocabularyProfile for PythonProfile {
    fn is_nominal_type(&self, annotation: &str) -> bool {
        super::typevocab::is_nominal_type(annotation)
    }

    fn loose_kind(&self, annotation: &str) -> Option<crate::profiles::typevocab::LooseTypeKind> {
        super::typevocab::loose_kind(annotation)
    }

    fn is_bool(&self, annotation: &str) -> bool {
        super::typevocab::is_bool(annotation)
    }

    fn is_dictish(&self, annotation: &str) -> bool {
        super::typevocab::is_dictish(annotation)
    }

    fn has_domain_model(&self, annotation: &str) -> bool {
        super::typevocab::has_domain_model(annotation)
    }

    fn is_primitive_scalar_alias(&self, annotation: &str) -> bool {
        super::typevocab::is_primitive_scalar_alias(annotation)
    }

    fn is_literal_union(&self, annotation: &str) -> bool {
        super::typevocab::is_literal_union(annotation)
    }

    fn admits_missing(&self, annotation: &str) -> bool {
        super::typevocab::admits_missing(annotation)
    }

    fn is_mutable_collection(&self, annotation: &str, family: MutatorFamily) -> bool {
        super::typevocab::is_mutable_collection(annotation, family)
    }

    fn is_void(&self, annotation: &str) -> bool {
        super::typevocab::is_void(annotation)
    }
}

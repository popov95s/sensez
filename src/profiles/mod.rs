//! Everything language-specific (grammar, tree→[`Walked`] walk, module naming,
//! import resolution, dead-code conventions) lives behind narrow profile traits.
//! The rest of sensez (graph algorithms, duplication, cycles, boundaries,
//! reporting) consumes only the language-neutral output types in [`crate::spine::parser`].
//!
//! Each language module is `#[cfg(feature)]`-gated so an unbuilt language
//! compiles to nothing.

pub(crate) mod branch_path;
pub(crate) mod comment_class;
pub(crate) mod comments;
pub(crate) mod conditionals;
pub mod dependencies;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod fallback_guard;
pub(crate) mod for_loop;
pub(crate) mod guard_clause;
pub(crate) mod guard_fingerprint;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod handler_end;
pub(crate) mod lexeme;
pub(crate) mod lineage;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod magic_number;
pub(crate) mod node_query;
#[cfg(any(feature = "lang-javascript", feature = "lang-rust"))]
pub(crate) mod pathroot;
pub mod registry;
pub(crate) mod runner_config;
#[cfg(test)]
pub(crate) mod test_fixtures;
pub(crate) mod test_oracle;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod todo_comment;
#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(crate) mod try_handling;
pub mod typevocab;
pub mod utility_lookalikes;
pub(crate) mod walk;
pub mod wording;

#[cfg(feature = "lang-javascript")]
pub mod javascript;
#[cfg(feature = "lang-python")]
pub mod python;
#[cfg(feature = "lang-rust")]
pub mod rust;
#[cfg(feature = "lang-typescript")]
pub mod typescript;

use crate::spine::ir::{
    ClassProperty, ClassUnit, ImportContext, ImportPhase, Language, MutatorFamily, PerfLine, Walked,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Compile built-in profile globs once at their call site rather than
/// duplicating filename conventions in consumers.
pub(crate) fn compile_profile_globs(patterns: &[&str]) -> globset::GlobSet {
    let compiled: Vec<String> = patterns
        .iter()
        .map(|pattern| (*pattern).to_string())
        .collect();
    crate::globs::build_globset(&compiled).expect("built-in profile globs must be valid")
}

/// Pure-data facts about a language. Cheap to reference; drives crawler
/// discovery and config defaults.
#[derive(Debug, Clone, Copy)]
pub struct LanguageInfo {
    pub language: Language,
    /// File extensions claimed by this profile, without the dot.
    pub extensions: &'static [&'static str],
    /// Test helpers, mocks, fixtures and stories: skipped by smells and duplication, still live for dead code.
    pub test_support: &'static [&'static str],
    /// Generated, often cyclic migration scripts: skipped by duplication, smells and cycles.
    pub migration_layouts: &'static [&'static str],
    /// Test runners that collect this language's tests; empty when reflexez cannot select them.
    pub test_runners: &'static [runner_config::RunnerKind],
    /// Smells this language switches off by default, before any `sensez.toml` overlay.
    pub default_off_smells: &'static [crate::report::SmellKind],
    pub wording: &'static wording::SmellWording,
}

/// Dead-code conventions supplied by a language profile.
///
/// User config remains global and explicit; these defaults are applied only to
/// files whose language owns the profile, so Python framework/test conventions
/// never leak into JS/TS/Rust findings in mixed repositories.
#[derive(Debug, Clone, Copy)]
pub struct DeadCodeDefaults {
    pub registration_decorators: &'static [&'static str],
    pub entrypoint_names: &'static [&'static str],
    /// Names live when they are class methods only (framework callbacks).
    pub method_entrypoint_names: &'static [&'static str],
    pub external_bases: ExternalBases,
    pub entrypoint_bases: &'static [&'static str],
    pub entry_globs: &'static [&'static str],
    /// Runner-executed files (test roots).
    pub test_sources: &'static [&'static str],
    /// Test helpers whose test importers count as users.
    pub test_support: &'static [&'static str],
    /// Tool config files: roots, and their imports are weaker evidence.
    pub runner_configs: &'static [&'static str],
}

impl DeadCodeDefaults {
    /// An all-empty `DeadCodeDefaults` — for profiles that don't override any
    /// convention (JS/TS today, anything new in the future). Spread it with
    /// `..DeadCodeDefaults::EMPTY` and only fill the fields that differ.
    #[cfg_attr(not(feature = "lang-rust"), allow(dead_code))]
    pub const EMPTY: DeadCodeDefaults = DeadCodeDefaults {
        registration_decorators: &[],
        entrypoint_names: &[],
        method_entrypoint_names: &[],
        external_bases: ExternalBases::Ignored,
        entrypoint_bases: &[],
        entry_globs: &[],
        test_sources: &[],
        test_support: &[],
        runner_configs: &[],
    };
}

/// Whether a class base that is not in the scan may call its methods (framework hooks).
#[derive(Debug, Clone, Copy)]
pub enum ExternalBases {
    /// Out-of-scan bases say nothing about method liveness.
    Ignored,
    /// Any out-of-scan base or interface may call methods.
    ImplyContract,
}

/// Result of classifying a symbol's decorator set for dead-code liveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoratorClass {
    /// No decorators (or the language has none).
    None,
    /// Structural/stdlib decorators only — no effect on liveness.
    Neutral,
    /// Framework-registration decorator present — treat the symbol as live.
    Registration,
    /// A bare unrecognized decorator — uncertain, downgrade confidence.
    Unknown,
}

impl DecoratorClass {
    pub(crate) fn is_registration(self) -> bool {
        matches!(self, Self::Registration)
    }

    pub(crate) fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown)
    }
}

pub trait ParseProfile: Send + Sync {
    fn info(&self) -> &'static LanguageInfo;
    fn ts_language(&self) -> tree_sitter::Language;
    fn walk(&self, root: tree_sitter::Node, src: &[u8], file_id: u32, module_name: &str) -> Walked;
    fn is_generated_or_data_source(&self, _path: &Path) -> bool {
        false
    }
    fn is_declaration_only(&self, _path: &Path) -> bool {
        false
    }
    /// Parsed but machine-written: kept in the graph, skipped by smells and duplication.
    fn is_generated(&self, _path: &Path, _src: &[u8]) -> bool {
        false
    }
    /// Languages sharing a clone family are compared as one duplication cohort.
    fn clone_family(&self) -> Language {
        self.info().language
    }
}

pub trait ModuleProfile: Send + Sync {
    /// Workspace/package root containing `file` (module names are relative to it).
    fn root_for(&self, file: &Path) -> PathBuf;
    /// Logical module name for `file` under `root` (dotted for Python, path-based for JS/TS/Rust).
    fn module_name(&self, file: &Path, root: &Path) -> String;
    /// True when `file` is the package's `__init__`/`index` entry.
    fn is_package_index(&self, file: &Path) -> bool;
    /// The containing package of `module_name` (`a.b.c` -> `a.b`; index files stay one level deeper).
    fn containing_package(&self, module_name: &str, is_index: bool) -> String;
    /// Fresh resolver state for one graph build.
    fn import_resolver(&self) -> Box<dyn ImportResolver>;
    /// Give a colliding module identity a profile-specific, workspace-relative
    /// name. Profiles that have no alternate namespace retain `base`.
    fn disambiguated_module_name(&self, file: &Path, workspace_root: &Path, base: &str) -> String {
        let _ = (file, workspace_root);
        base.to_string()
    }
    /// Workspace above `common` whose layout disambiguates colliding module names.
    fn workspace_root(&self, _common: &Path) -> Option<PathBuf> {
        None
    }
    fn submodule_candidate(&self, target: &str, symbol: &str) -> Option<String>;
    fn is_containment(&self, _importer: &str, _target: &str) -> bool {
        false
    }
}

/// Import resolution owned by one language; lives for one graph build so cached inputs never go stale.
pub trait ImportResolver {
    fn resolve_target(
        &mut self,
        import: &ImportContext,
        importer_package: &str,
        importer_file: &Path,
        root: &Path,
    ) -> String;
    /// Phase of the graph edge built from `import`.
    fn edge_phase(&mut self, import: &ImportContext, _importer_file: &Path) -> ImportPhase {
        import.phase
    }
    /// Filesystem path an aliased import specifier maps to, for modules under `root`.
    fn alias_path(&mut self, _root: &Path, _specifier: &str) -> Option<PathBuf> {
        None
    }
}

pub trait DeadCodeProfile: Send + Sync {
    fn classify_decorator(
        &self,
        paths: Option<&Vec<String>>,
        user_entrypoints: &HashSet<String>,
    ) -> DecoratorClass;
    /// True when `symbol` is *uninteresting* dead code by language convention
    /// (underscore-prefix in Python/Rust, `test_` prefix in Python, etc.) and
    /// should be skipped from dead-code findings. **The exact meaning is
    /// per-language**:
    ///
    /// - Python: `_<name>` or `test_<name>` (private/test).
    /// - Rust: `<name>` starting with `_` (intentionally unused binding).
    /// - JS/TS: never — no enforced convention.
    ///
    /// Reaching this method from a smell detector means a symbol the
    /// reachability pass already failed to credit through the import graph;
    /// the language convention decides whether to *suppress* that finding
    /// entirely (true) or surface it at the default confidence (false).
    fn is_conventionally_private(&self, symbol: &str) -> bool;
    fn is_entry_file_stem(&self, stem: &str) -> bool;
    fn manages_class_properties(
        &self,
        _class: &ClassUnit,
        _decorators: Option<&Vec<String>>,
    ) -> bool {
        false
    }
    fn manages_property(&self, _property: &ClassProperty) -> bool {
        false
    }
    fn requires_property_usage_evidence(&self, _class: &ClassUnit) -> bool {
        false
    }
    fn dead_code_defaults(&self) -> DeadCodeDefaults;
    /// Module keys a project manifest names as entry points.
    fn entry_modules(&self, _project_root: &Path) -> Vec<String> {
        Vec::new()
    }
    /// Files a package manifest names as entry points.
    fn manifest_entry_files(&self, _files: &[&Path]) -> Vec<PathBuf> {
        Vec::new()
    }
}

pub trait PerformanceProfile: Send + Sync {
    fn receiver_root<'a>(&self, receiver: &'a str) -> &'a str;
    fn is_mutating_call(&self, method: &str) -> bool;
    fn is_bounded_loop(&self, subject: &str) -> bool;
    fn is_external_loop_call(
        &self,
        method: &str,
        receiver: &str,
        receiver_type: Option<&str>,
        loops: &[PerfLine],
    ) -> bool;
}

/// Language-owned interpretation of type annotations used by smell detectors.
pub trait TypeVocabularyProfile: Send + Sync {
    fn loose_kind(&self, annotation: &str) -> Option<typevocab::LooseTypeKind>;
    fn is_bool(&self, annotation: &str) -> bool;
    fn is_dictish(&self, annotation: &str) -> bool;
    /// A dict-ish annotation whose keys are typed or declared open, so literal keys are not a hidden schema.
    fn is_open_map(&self, _annotation: &str) -> bool {
        false
    }
    fn has_domain_model(&self, annotation: &str) -> bool;
    fn is_primitive_scalar_alias(&self, annotation: &str) -> bool;
    /// Param a type-guard or assertion return annotation narrows (`p is T`).
    fn guarded_param<'a>(&self, _return_annotation: &'a str) -> Option<&'a str> {
        None
    }
    /// Annotation that already restricts values to a closed set of string literals.
    fn is_literal_union(&self, _annotation: &str) -> bool {
        false
    }
    /// Whether a value of this annotation is mutated by a `family` method call.
    fn is_mutable_collection(&self, annotation: &str, family: MutatorFamily) -> bool;
    fn is_void(&self, annotation: &str) -> bool;
    /// The annotation admits a missing value (`T | null`, `Optional[T]`).
    fn admits_missing(&self, _annotation: &str) -> bool {
        false
    }
    /// A named type (not an inline literal, union or intersection) that behaviour could move onto.
    fn is_nominal_type(&self, annotation: &str) -> bool {
        typevocab::is_nominal(annotation, &[])
    }
}

pub use dependencies::DependencyProfile;

pub trait LanguageProfile:
    ParseProfile
    + ModuleProfile
    + DeadCodeProfile
    + PerformanceProfile
    + TypeVocabularyProfile
    + DependencyProfile
{
}

impl<T> LanguageProfile for T where
    T: ParseProfile
        + ModuleProfile
        + DeadCodeProfile
        + PerformanceProfile
        + TypeVocabularyProfile
        + DependencyProfile
{
}

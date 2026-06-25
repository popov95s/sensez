//! Everything language-specific (grammar, tree→[`Walked`] walk, module naming,
//! import resolution, dead-code conventions) lives behind narrow profile traits.
//! The rest of sensez (graph algorithms, duplication, cycles, boundaries,
//! reporting) consumes only the language-neutral output types in [`crate::spine::parser`].
//!
//! Each language module is `#[cfg(feature)]`-gated so an unbuilt language
//! compiles to nothing.

pub(crate) mod conditionals;
pub(crate) mod lexeme;
#[cfg(any(feature = "lang-javascript", feature = "lang-rust"))]
pub(crate) mod pathroot;
pub mod registry;
pub mod typevocab;
pub(crate) mod walk;

#[cfg(feature = "lang-javascript")]
pub mod javascript;
#[cfg(feature = "lang-python")]
pub mod python;
#[cfg(feature = "lang-rust")]
pub mod rust;
#[cfg(feature = "lang-typescript")]
pub mod typescript;

use crate::spine::ir::{ImportContext, Language, Walked};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Pure-data facts about a language. Cheap to reference; drives crawler
/// discovery and config defaults.
#[derive(Debug, Clone, Copy)]
pub struct LanguageInfo {
    pub language: Language,
    /// File extensions claimed by this profile, without the dot.
    pub extensions: &'static [&'static str],
}

/// Dead-code conventions supplied by a language profile.
///
/// User config remains global and explicit; these defaults are applied only to
/// files whose language owns the profile, so Python framework/test conventions
/// never leak into JS/TS/Rust findings in mixed repositories.
#[derive(Debug, Clone, Copy)]
pub struct DeadCodeDefaults {
    pub entrypoints: &'static [&'static str],
    pub entrypoint_names: &'static [&'static str],
    pub entrypoint_bases: &'static [&'static str],
    pub entry_points: &'static [&'static str],
    pub test_sources: &'static [&'static str],
}

impl DeadCodeDefaults {
    /// An all-empty `DeadCodeDefaults` — for profiles that don't override any
    /// convention (JS/TS today, anything new in the future). Spread it with
    /// `..DeadCodeDefaults::EMPTY` and only fill the fields that differ.
    pub const EMPTY: DeadCodeDefaults = DeadCodeDefaults {
        entrypoints: &[],
        entrypoint_names: &[],
        entrypoint_bases: &[],
        entry_points: &[],
        test_sources: &[],
    };
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
}

pub trait ModuleProfile: Send + Sync {
    fn root_for(&self, file: &Path) -> PathBuf;
    fn module_name(&self, file: &Path, root: &Path) -> String;
    fn is_package_index(&self, file: &Path) -> bool;
    fn containing_package(&self, module_name: &str, is_index: bool) -> String;
    fn resolve_target(
        &self,
        import: &ImportContext,
        importer_package: &str,
        importer_file: &Path,
        root: &Path,
    ) -> String;
    fn submodule_candidate(&self, target: &str, symbol: &str) -> Option<String>;
    fn is_containment(&self, _importer: &str, _target: &str) -> bool {
        false
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
    fn dead_code_defaults(&self) -> DeadCodeDefaults;
    fn entry_modules(&self, project_root: &Path) -> Vec<String>;
}

pub trait PerformanceProfile: Send + Sync {
    fn is_expensive_loop_call(&self, method: &str) -> bool;
    fn is_external_get_receiver(&self, base: &str) -> bool;
}

pub trait LanguageProfile:
    ParseProfile + ModuleProfile + DeadCodeProfile + PerformanceProfile
{
}

impl<T> LanguageProfile for T where
    T: ParseProfile + ModuleProfile + DeadCodeProfile + PerformanceProfile
{
}

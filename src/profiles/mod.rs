//! Everything language-specific (grammar, tree→[`Walked`] walk, module naming,
//! import resolution, dead-code conventions) lives behind narrow profile traits.
//! The rest of sensez (graph algorithms, duplication, cycles, boundaries,
//! reporting) consumes only the language-neutral output types in [`crate::spine::parser`].
//!
//! Each language module is `#[cfg(feature)]`-gated so an unbuilt language
//! compiles to nothing.

pub(crate) mod lexeme;
#[cfg(any(feature = "lang-javascript", feature = "lang-rust"))]
pub(crate) mod pathroot;
pub(crate) mod profile_macro;
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

use crate::spine::ir::{ImportContext, Walked};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Stable identity of a supported language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
}

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
}

impl DeadCodeDefaults {
    #[cfg(feature = "lang-rust")]
    pub const EMPTY: DeadCodeDefaults = DeadCodeDefaults {
        entrypoints: &[],
        entrypoint_names: &[],
        entrypoint_bases: &[],
        entry_points: &[],
    };
}

/// Result of classifying a symbol's decorator set by shape.
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
    fn is_conventionally_private(&self, symbol: &str) -> bool;
    fn is_entry_file_stem(&self, stem: &str) -> bool;
    fn dead_code_defaults(&self) -> DeadCodeDefaults;
    fn entry_modules(&self, project_root: &Path) -> Vec<String>;
}

pub trait LanguageProfile: ParseProfile + ModuleProfile + DeadCodeProfile {}

impl<T> LanguageProfile for T where T: ParseProfile + ModuleProfile + DeadCodeProfile {}

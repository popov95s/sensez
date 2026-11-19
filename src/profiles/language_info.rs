use super::runner_config;
use super::wording;
use crate::spine::ir::Language;

/// Pure-data facts about a language.
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
    /// File-name globs of project and compiler config files whose edits change findings.
    #[cfg(feature = "lsp")]
    pub project_files: &'static [&'static str],
    pub import_words: &'static [&'static str],
    pub wording: &'static wording::SmellWording,
}

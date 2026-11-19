use super::external_bases::ExternalBases;

/// Dead-code conventions supplied by a language profile.
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
    /// All-empty defaults; spread it with `..DeadCodeDefaults::EMPTY`.
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

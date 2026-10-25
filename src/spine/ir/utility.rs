//! Functions shaped like a well-known utility that the project already depends on.

/// How the project already has a utility's provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderOrigin {
    StandardLibrary,
    /// Declared by the named manifest file.
    Manifest(String),
    /// Listed by the `dependency_reimplementation_providers` knob.
    Configured,
}

/// A function the project could call instead, and why it is available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilityProvider {
    pub module: String,
    pub function: String,
    pub origin: ProviderOrigin,
}

/// Parameter range and body size of a utility's typical hand-written copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtilityShape {
    pub min_arity: usize,
    pub max_arity: usize,
    pub max_lines: usize,
}

/// A module-level function named like a known utility whose providers the project declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtilityLookalike {
    pub name: String,
    /// The utility's canonical name, shared by its aliases.
    pub utility: &'static str,
    pub line: usize,
    pub arity: usize,
    pub lines: usize,
    pub shape: UtilityShape,
    /// Providers present for this file, best suggestion first.
    pub providers: Vec<UtilityProvider>,
}

//! Test doubles a test file installs over a module or one of its members.

/// What the double names as its host.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MockHost {
    /// A module specifier written as a string (`"./pricing"`, `"pkg.pricing"`).
    Specifier(String),
    /// A local name bound to an imported object (`patch.object(Pricing, ...)`).
    Binding(String),
}

/// How much of the host the double replaces.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MockScope {
    WholeModule,
    Member(String),
}

/// Whether the double keeps the real host behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MockFidelity {
    Replaces,
    KeepsOriginal,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MockTarget {
    pub line: usize,
    pub host: MockHost,
    pub scope: MockScope,
    pub fidelity: MockFidelity,
}

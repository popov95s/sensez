//! The shared output types every language walker produces and every analyzer
//! pillar consumes ([`Walked`], [`ImportContext`], [`StructuralToken`], the
//! per-function/class units). Both `crate::profiles` (producers) and
//! `crate::spine::parser` (dispatch) depend on this module — neither on the other's
//! internals — which keeps the profile seam acyclic.

mod import_context;
mod performance;
pub mod tokens;
mod units;

#[cfg(feature = "eyez")]
use crate::eyez::RawDoc;
pub use import_context::{ImportContext, ImportPhase};
pub use performance::{CallFact, PerfLine, PerformanceFacts};
pub use tokens::{StructuralToken, TokenSpan};
pub(crate) use units::{bump, record_attr};
pub use units::{ClassProperty, ClassUnit, FunctionMetrics, FunctionUnit, TypeHints};

use std::collections::{HashMap, HashSet};

/// What kind of source symbol a declaration (or dead-code finding) refers to.
/// Serialized snake_case, so the JSON output is identical to the historical
/// string values ("function", "class", ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Class,
    Variable,
    Method,
    Import,
}

impl SymbolKind {
    /// The serialized name (matches the serde representation).
    pub fn as_str(self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Class => "class",
            SymbolKind::Variable => "variable",
            SymbolKind::Method => "method",
            SymbolKind::Import => "import",
        }
    }
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Result of walking one file's syntax tree — the language-neutral projection
/// every analyzer pillar reads. Languages that lack a concept (e.g. JS has no
/// `__all__`) simply leave the corresponding field empty/`None`.
#[derive(Debug, Default)]
pub struct SyntaxFacts {
    pub tokens: Vec<StructuralToken>,
    pub spans: Vec<TokenSpan>,
    pub lexemes: Vec<u64>,
}

#[derive(Debug, Default)]
pub struct SymbolFacts {
    pub imports: Vec<ImportContext>,
    pub declared: Vec<String>,
    pub declared_kinds: HashMap<String, SymbolKind>,
    pub declared_lines: HashMap<String, usize>,
    pub methods: Vec<(String, usize)>,
    pub dunder_all: Option<Vec<String>>,
    pub decorators: HashMap<String, Vec<String>>,
}

#[derive(Debug, Default)]
pub struct UsageFacts {
    pub name_counts: HashMap<String, usize>,
    pub attribute_accesses: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Default)]
pub struct UnitFacts {
    pub functions: Vec<FunctionUnit>,
    pub classes: Vec<ClassUnit>,
    pub type_hints: TypeHints,
}

#[derive(Debug, Default)]
pub struct Walked {
    /// Structural-token view used by duplication and diff-aware span mapping.
    pub syntax: SyntaxFacts,
    /// Top-level declarations, imports, and module export facts.
    pub symbols: SymbolFacts,
    /// Intra-module usage evidence and attribute access facts.
    pub usage: UsageFacts,
    /// Function/class summaries consumed by the smell detectors.
    pub units: UnitFacts,
    /// Docstrings + comments for the eyez index. Populated only under the
    /// `eyez` feature; never feeds the structural token stream.
    #[cfg(feature = "eyez")]
    pub docs: Vec<RawDoc>,
}

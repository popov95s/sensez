//! The shared output types every language walker produces and every analyzer
//! pillar consumes ([`Walked`], [`ImportContext`], [`StructuralToken`], the
//! per-function/class units). Both `crate::profiles` (producers) and
//! `crate::spine::parser` (dispatch) depend on this module — neither on the other's
//! internals — which keeps the profile seam acyclic.

mod echo;
mod fallback;
mod import_context;
mod lang;
mod loop_frame;
mod magic_number;
mod mock_target;
mod performance;
mod sentinel_handler;
mod state_write;
pub mod tokens;
mod type_escape;
mod units;
mod utility;

#[cfg(feature = "eyez")]
use crate::eyez::RawDoc;
pub use echo::{EchoAssertion, EchoShape};
pub use fallback::{FallbackSink, FallbackSite, StringFallback};
pub use import_context::{ImportContext, ImportPhase};
pub use lang::Language;
pub use loop_frame::{LoopBound, LoopFrame, LoopKind};
pub use magic_number::{MagicContext, MagicNumber};
pub use mock_target::{MockFidelity, MockHost, MockScope, MockTarget};
pub use performance::{CallFact, IterationPass, PerfLine, PerformanceFacts};
pub use sentinel_handler::{GuardedBody, HandlerComment, HandlerEnd, SentinelHandler};
pub use state_write::{StateWrite, StateWriteKind};
pub use tokens::{StructuralToken, TokenSpan};
pub use type_escape::{TypeEscape, TypeEscapeKind};
pub(crate) use units::{bump, record_attr};
pub use units::{
    BranchPath, CallbackPosition, ChainHit, ClassProperty, ClassUnit, CommentClass, CommentSpan,
    FunctionKey, FunctionUnit, LiteralMembership, MutatorFamily, NestedTernary, SchemaCall,
    StubBody, TypeAlias, TypeHints, Visibility, ANONYMOUS,
};
pub use utility::{ProviderOrigin, UtilityLookalike, UtilityProvider, UtilityShape};

use std::collections::{HashMap, HashSet};

/// What kind of source symbol a declaration (or dead-code finding) refers to.
/// Serialized snake_case, so the JSON output is identical to the historical
/// string values ("function", "class", ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Class,
    Variable,
    Method,
    Import,
    Property,
    /// A whole module, for findings about modules rather than one symbol.
    Module,
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
            SymbolKind::Property => "property",
            SymbolKind::Module => "module",
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
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SyntaxFacts {
    pub tokens: Vec<StructuralToken>,
    pub spans: Vec<TokenSpan>,
    pub lexemes: Vec<u64>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SymbolFacts {
    pub imports: Vec<ImportContext>,
    pub declared: Vec<String>,
    pub declared_kinds: HashMap<String, SymbolKind>,
    pub declared_lines: HashMap<String, usize>,
    pub methods: Vec<(String, usize)>,
    pub dunder_all: Option<Vec<String>>,
    pub decorators: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub default_export: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UsageFacts {
    pub name_counts: HashMap<String, usize>,
    pub attribute_accesses: HashMap<String, HashSet<String>>,
    pub attribute_path_accesses: HashMap<String, HashSet<String>>,
    pub call_result_attribute_accesses: HashMap<String, HashSet<String>>,
    pub string_literals: HashSet<String>,
    pub chained_attribute_names: HashSet<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UnitFacts {
    pub functions: Vec<FunctionUnit>,
    pub classes: Vec<ClassUnit>,
    pub type_hints: TypeHints,
    pub comment_spans: Vec<CommentSpan>,
    /// Test doubles installed by module-mocking and patching calls.
    pub mock_targets: Vec<MockTarget>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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

impl Walked {
    /// Drops the spare capacity of the per-token and per-function vectors held for the whole scan.
    pub fn shrink_to_fit(&mut self) {
        self.syntax.tokens.shrink_to_fit();
        self.syntax.spans.shrink_to_fit();
        self.syntax.lexemes.shrink_to_fit();
        self.units.functions.shrink_to_fit();
    }
}

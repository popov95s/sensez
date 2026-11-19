use super::{
    ClassUnit, CommentSpan, FunctionUnit, ImportContext, MockTarget, StructuralToken, SymbolKind,
    TokenSpan, TypeHints,
};
#[cfg(feature = "eyez")]
use crate::eyez::RawDoc;
use std::collections::{HashMap, HashSet};

/// Language-neutral result of walking one file; concepts a language lacks stay empty.
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
    pub declared_lines: rustc_hash::FxHashMap<String, usize>,
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
    /// Docstrings and comments for the eyez index; never feeds the token stream.
    #[cfg(feature = "eyez")]
    pub docs: Vec<RawDoc>,
}

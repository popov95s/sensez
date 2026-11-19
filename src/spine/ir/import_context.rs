//! Language-neutral import data model (populated by each language's walker).

/// One resolved import target; `import a, b` yields two contexts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportContext {
    pub source_module: String,
    pub target_module: String,
    pub imported_symbols: Vec<String>,
    /// Local names this import binds (alias-aware): `import a.b` binds `a`.
    pub bindings: Vec<String>,
    /// Per-binding phase, in the same order and with the same length as `bindings`.
    pub binding_phases: Vec<ImportPhase>,
    pub line: usize,
    pub column: usize,
    pub phase: ImportPhase,
    /// True if found inside a function/class body (not at module top level).
    pub is_inline: bool,
    /// Rust `mod x;` declaration: containment, not coupling, so excluded from cycle detection.
    pub is_module_decl: bool,
    /// True for a re-export (`export { x } from "./y"`): forwards names, binds none locally.
    #[serde(default)]
    pub is_reexport: bool,
    /// Name of the nearest enclosing function/class, if any.
    pub enclosing_scope: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhase {
    Runtime,
    TypeOnly,
}

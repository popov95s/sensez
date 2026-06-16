//! Language-neutral import data model (populated by each language's walker).

/// Context for a single resolved import target.
///
/// `import a, b` yields two `ImportContext`s; `from x import a, b` yields one
/// with `imported_symbols = ["a", "b"]`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportContext {
    pub source_module: String,
    pub target_module: String,
    pub imported_symbols: Vec<String>,
    /// Local names this import binds into the file (alias-aware). Used for
    /// unused-import detection. `import a.b` binds `a`; `x as y` binds `y`.
    pub bindings: Vec<String>,
    pub line: usize,
    pub column: usize,
    /// True if found inside a function/class body (not at module top level).
    pub is_inline: bool,
    /// True for a module-hierarchy declaration (Rust `mod x;`): containment,
    /// not coupling. Counts as usage (the child is reachable) but is excluded
    /// from cycle detection — parent declares child + child uses `super::` is
    /// the idiomatic Rust layout, not a circular dependency.
    pub is_module_decl: bool,
    /// Name of the nearest enclosing function/class, if any.
    pub enclosing_scope: Option<String>,
}

//! Documentation elements (docstrings + comments) captured during the language
//! walk for the optional eyez index. Pure data — no ML, no embeddings here.
//!
//! Populated only under the `eyez` feature, inside each language's walk,
//! and written *only* to [`Walked::docs`](crate::spine::parser::Walked) — never to the
//! structural `tokens`/`lexemes`/`spans` the duplication pillar consumes. The
//! whole module compiles out when the feature is off.

/// What kind of documentation a [`RawDoc`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DocKind {
    /// A docstring: the bare string literal opening a module/class/function body.
    Docstring,
    /// A source comment (`#`, `//`, `/* … */`).
    Comment,
}

/// One documentation element mapped to the symbol it documents.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawDoc {
    /// Fully-qualified owner: `module::dotted.symbol.path` (e.g.
    /// `billing::Webhook.verify`), or the module name alone for a module-level
    /// doc / a comment with no enclosing symbol.
    pub symbol_path: String,
    pub kind: DocKind,
    /// Literal documentation text (string contents / comment body).
    pub text: String,
    /// 1-indexed source line of the doc element (the docstring/comment itself).
    pub line: usize,
}

impl RawDoc {
    /// Build a [`RawDoc`], composing the fully-qualified `symbol_path` from the
    /// module name and the dotted scope path (empty scope ⇒ module-level).
    pub fn new(
        module: &str,
        scope_path: &[&str],
        kind: DocKind,
        text: String,
        line: usize,
    ) -> Self {
        let symbol_path = if scope_path.is_empty() {
            module.to_string()
        } else {
            format!("{module}::{}", scope_path.join("."))
        };
        RawDoc {
            symbol_path,
            kind,
            text,
            line,
        }
    }
}

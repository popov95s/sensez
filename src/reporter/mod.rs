//! The [`AnalysisReport`] (see [`crate::noze`]) is the single source of
//! truth. JSON is the machine contract; the terminal renderer is a human view
//! over the very same data.

mod json;
mod terminal;

pub use json::to_json;
pub use terminal::render;

/// Output format selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Terminal,
    Json,
}

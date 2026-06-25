//! The [`AnalysisReport`] (see [`crate::noze`]) is the single source of
//! truth. JSON is the machine contract; the terminal renderer is a human
//! view; the path filter is a narrower slice. All three live here.

mod filter;
mod json;
mod terminal;

pub use filter::apply;
pub use json::to_json;
pub use terminal::render;

/// Output format selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Terminal,
    Json,
}

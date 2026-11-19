//! The [`AnalysisReport`] (see [`crate::noze`]) is the single source of truth.

mod diagnostics;
mod filter;
mod format;
mod json;
mod terminal;

pub use diagnostics::ScanDiagnostics;
pub use filter::apply;
pub use format::Format;
pub use json::to_json;
pub use terminal::{render, Glossary};

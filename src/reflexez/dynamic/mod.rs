//! On-demand dynamic-import extraction, never reached by Noze scans.

#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
mod context;
mod facts;
#[cfg(feature = "lang-javascript")]
pub(super) mod javascript;
#[cfg(feature = "lang-python")]
mod python;
mod scan;

#[cfg(any(feature = "lang-javascript", feature = "lang-python"))]
pub(super) use context::context;
pub use facts::{DynamicFacts, FileFacts};
pub use scan::{may_import, scan};

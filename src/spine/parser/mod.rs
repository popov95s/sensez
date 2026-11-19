mod batch;
mod depth;
mod loaded_source;
mod parsed_file;
pub(crate) mod timing;
mod tree;

pub use crate::spine::ir::tokens;
pub use crate::spine::ir::{ClassProperty, FunctionUnit, ImportContext, ImportPhase, SymbolKind};
pub use batch::{parse_files, parse_sources, parse_sources_incremental};
#[cfg(test)]
pub use loaded_source::parse_file;
pub use parsed_file::{ParseBatch, ParsedFile};
#[cfg(test)]
pub use tree::parse_source;

#[cfg(test)]
mod tests;

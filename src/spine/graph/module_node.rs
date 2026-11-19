use crate::spine::ir::Language;
use std::path::PathBuf;

/// A module in the codebase (or a synthetic external dependency).
#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub file_path: PathBuf,
    pub module_name: String,
    /// Language of the source file; external nodes inherit the importer's.
    pub language: Language,
    /// Index into the immutable parsed-file table; absent for external nodes.
    pub source_index: Option<usize>,
    /// True for stdlib/third-party/unresolved targets outside the scan tree.
    pub is_external: bool,
}

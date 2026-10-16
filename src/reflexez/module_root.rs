use crate::spine::ir::Language;
use std::path::{Path, PathBuf};

/// The root a file's graph keys are relative to, matching the graph builder's choice.
pub fn module_root(file: &Path, language: Language, configured: &[PathBuf]) -> PathBuf {
    configured
        .iter()
        .filter(|root| file.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned()
        .unwrap_or_else(|| crate::profiles::registry::module_profile(language).root_for(file))
}

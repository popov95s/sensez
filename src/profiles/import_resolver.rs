use crate::spine::ir::{ImportContext, ImportPhase};
use std::path::{Path, PathBuf};

/// Import resolution owned by one language; lives for one graph build.
pub trait ImportResolver {
    fn resolve_target(
        &mut self,
        import: &ImportContext,
        importer_package: &str,
        importer_file: &Path,
        root: &Path,
    ) -> String;
    /// Phase of the graph edge built from `import`.
    fn edge_phase(&mut self, import: &ImportContext, _importer_file: &Path) -> ImportPhase {
        import.phase
    }
    /// Candidate module path keys of `import`, tried in order before [`Self::resolve_target`].
    fn target_path_keys(&mut self, _import: &ImportContext, _importer_file: &Path) -> Vec<PathBuf> {
        Vec::new()
    }
    /// Filesystem path an aliased import specifier of `importer_file` maps to.
    fn alias_path(&mut self, _importer_file: &Path, _specifier: &str) -> Option<PathBuf> {
        None
    }
}

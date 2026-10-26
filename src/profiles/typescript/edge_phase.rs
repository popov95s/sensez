//! Graph phase of TypeScript imports whose specifiers are all inline `type`.

use super::paths::ResolutionCache;
use super::type_imports::TypeOnlyImports;
use crate::spine::ir::{ImportContext, ImportPhase};
use std::path::Path;

/// Graph phase of an import whose specifiers are all inline `type`.
pub(super) fn edge_phase(
    cache: &mut ResolutionCache,
    import: &ImportContext,
    importer_file: &Path,
) -> ImportPhase {
    let all_type = !import.binding_phases.is_empty()
        && import
            .binding_phases
            .iter()
            .all(|phase| *phase == ImportPhase::TypeOnly);
    if import.phase == ImportPhase::TypeOnly || !all_type {
        return import.phase;
    }
    let dir = importer_file.parent().unwrap_or(Path::new(""));
    match cache.type_only_imports(dir) {
        TypeOnlyImports::Erased => ImportPhase::TypeOnly,
        TypeOnlyImports::Kept => ImportPhase::Runtime,
    }
}

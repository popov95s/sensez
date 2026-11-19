//! Exact source-state confirmation for Brainz recapture candidates.

use super::hub;
use std::path::Path;

/// Confirm an mtime candidate with the same path/content manifest used by the parser cache.
pub(super) fn changed(root: &Path, baseline: &hub::Baseline) -> bool {
    let Some(previous) = baseline.source_state.as_ref() else {
        return true;
    };
    let Some(current) = current(root) else {
        return true;
    };
    previous.changes(&current).has_changes()
}

fn current(root: &Path) -> Option<crate::source_state::SourceState> {
    let Ok(config) = crate::config::model::Config::load(root) else {
        return None;
    };
    let Ok(discovery) = crate::spine::crawler::discover(root, &config.exclude, &|path| {
        crate::profiles::registry::should_parse_path(path)
    }) else {
        return None;
    };
    let Ok(project) = crate::spine::cache::load_project(&discovery.files) else {
        return None;
    };
    let manifest = crate::source_state::SourceManifest::from_root_hashes(
        root,
        project
            .sources
            .into_iter()
            .map(|source| (source.path, source.content_hash)),
    );
    Some(crate::source_state::SourceState::new(
        1,
        config.signature(),
        hub::branch_key(root),
        manifest,
    ))
}

#[cfg(test)]
#[path = "recapture_state_tests.rs"]
mod tests;

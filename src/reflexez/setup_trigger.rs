use super::runner_config::SetupFile;
use crate::spine::graph::CodebaseGraph;
use crate::spine::impact::{reachable_files, DirectionOfImpact, ImpactOptions};
use std::path::{Path, PathBuf};

/// Fallback reasons for changes to a configured setup file or anything it imports.
pub fn setup_fallback(
    graph: &CodebaseGraph,
    setup_files: &[SetupFile],
    changed: &[PathBuf],
    repository: &Path,
) -> Vec<String> {
    setup_files
        .iter()
        .filter(|setup| {
            let closure = reachable_files(
                graph,
                std::slice::from_ref(&setup.path),
                DirectionOfImpact::Dependencies,
                ImpactOptions {
                    include_type_only: false,
                },
            );
            changed
                .iter()
                .any(|path| *path == setup.path || closure.files.contains_key(path))
        })
        .map(|setup| {
            format!(
                "test setup {} ({} in {}) or one of its imports changed",
                relative(&setup.path, repository),
                setup.key,
                relative(&setup.config, repository)
            )
        })
        .collect()
}

fn relative(path: &Path, repository: &Path) -> String {
    path.strip_prefix(repository)
        .unwrap_or(path)
        .display()
        .to_string()
}

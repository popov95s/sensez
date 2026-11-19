use super::reachability::{reachable_files, DirectionOfImpact, ImpactOptions, Reachability};
use crate::spine::graph::CodebaseGraph;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Both directions of impact for a source change.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AffectedFiles {
    pub dependencies: Reachability,
    pub dependents: Reachability,
}

impl AffectedFiles {
    /// Union of changed paths, callers and callees, keeping the shortest distance.
    pub fn all_files(&self) -> BTreeMap<PathBuf, usize> {
        let mut files = self.dependencies.files.clone();
        for (path, distance) in &self.dependents.files {
            files
                .entry(path.clone())
                .and_modify(|current| *current = (*current).min(*distance))
                .or_insert(*distance);
        }
        files
    }

    pub fn unmapped(&self) -> BTreeSet<PathBuf> {
        self.dependencies
            .unmapped
            .union(&self.dependents.unmapped)
            .cloned()
            .collect()
    }
}

/// Compute the transitive caller and callee closure from a common graph.
pub fn affected_files(
    graph: &CodebaseGraph,
    changed: &[PathBuf],
    options: ImpactOptions,
) -> AffectedFiles {
    AffectedFiles {
        dependencies: reachable_files(graph, changed, DirectionOfImpact::Dependencies, options),
        dependents: reachable_files(graph, changed, DirectionOfImpact::Dependents, options),
    }
}

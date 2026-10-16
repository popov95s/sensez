use super::changes::{global_trigger, is_source};
use super::discovery::TestFile;
use super::selector_dynamic::{walk_reverse, Reach};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::path::PathBuf;

const CHANGED: Reach = Reach {
    distance: 0,
    dynamic: false,
};
/// A file with an opaque import may load the changed file, one hop away.
const OPAQUE_IMPORTER: Reach = Reach {
    distance: 1,
    dynamic: true,
};

/// Everything needed to walk from changed files to the tests they can affect.
pub(super) struct ChangeReach<'a> {
    pub tests: &'a HashMap<PathBuf, &'a TestFile>,
    pub nodes: &'a HashMap<PathBuf, NodeIndex>,
    pub reverse: &'a HashMap<NodeIndex, Vec<(NodeIndex, bool)>>,
    pub assets: &'a HashMap<PathBuf, Vec<NodeIndex>>,
    pub opaque_files: &'a [PathBuf],
}

pub(super) struct Reached {
    pub selected: HashMap<PathBuf, Reach>,
    pub unmapped: Vec<PathBuf>,
    pub missing_source: bool,
}

impl ChangeReach<'_> {
    pub fn reach(&self, changed: &[PathBuf]) -> Reached {
        let mut reached = Reached {
            selected: HashMap::new(),
            unmapped: Vec::new(),
            missing_source: false,
        };
        let mut seeds = Vec::new();
        for path in changed {
            if self.tests.contains_key(path) {
                reached.selected.insert(path.clone(), CHANGED);
            }
            if let Some(&node) = self.nodes.get(path) {
                seeds.push((node, CHANGED));
            } else if let Some(assets) = self.assets.get(path) {
                seeds.extend(assets.iter().map(|asset| (*asset, CHANGED)));
            } else if !path.exists() {
                reached.missing_source |= is_source(path);
            } else if global_trigger(path).is_none() {
                reached.unmapped.push(path.clone());
            }
        }
        if changed.iter().any(|path| is_source(path)) {
            seeds.extend(
                self.opaque_files
                    .iter()
                    .filter_map(|path| self.nodes.get(path))
                    .map(|node| (*node, OPAQUE_IMPORTER)),
            );
        }
        let test_nodes: HashMap<_, _> = self
            .tests
            .keys()
            .filter_map(|path| self.nodes.get(path).map(|index| (*index, path.clone())))
            .collect();
        walk_reverse(seeds, self.reverse, &test_nodes, &mut reached.selected);
        reached
    }
}

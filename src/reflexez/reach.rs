use super::changes::{global_trigger, is_source};
use super::discovery::TestFile;
use super::selector_dynamic::{walk_reverse, Reach};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

#[derive(Default)]
pub(super) struct Reached {
    pub selected: HashMap<PathBuf, Reach>,
    pub unmapped: Vec<PathBuf>,
    pub missing_source: bool,
}

enum Placement<'a> {
    Graph(NodeIndex),
    Assets(&'a [NodeIndex]),
    MissingSource,
    Unmapped,
    Ignored,
}

impl ChangeReach<'_> {
    pub fn reach(&self, changed: &[PathBuf]) -> Reached {
        let mut reached = Reached::default();
        let mut seeds = Vec::new();
        for path in changed {
            if self.tests.contains_key(path) {
                reached.selected.insert(path.clone(), CHANGED);
            }
            match self.place(path) {
                Placement::Graph(node) => seeds.push((node, CHANGED)),
                Placement::Assets(assets) => {
                    seeds.extend(assets.iter().map(|asset| (*asset, CHANGED)))
                }
                Placement::MissingSource => reached.missing_source = true,
                Placement::Unmapped => reached.unmapped.push(path.clone()),
                Placement::Ignored => {}
            }
        }
        if changed.iter().any(|path| is_source(path)) {
            seeds.extend(self.opaque_importers());
        }
        walk_reverse(
            seeds,
            self.reverse,
            &self.test_nodes(),
            &mut reached.selected,
        );
        reached
    }

    fn place(&self, path: &Path) -> Placement<'_> {
        if let Some(&node) = self.nodes.get(path) {
            return Placement::Graph(node);
        }
        if let Some(assets) = self.assets.get(path) {
            return Placement::Assets(assets);
        }
        if !path.exists() {
            return if is_source(path) {
                Placement::MissingSource
            } else {
                Placement::Ignored
            };
        }
        if global_trigger(path).is_some() {
            return Placement::Ignored;
        }
        Placement::Unmapped
    }

    fn opaque_importers(&self) -> impl Iterator<Item = (NodeIndex, Reach)> + '_ {
        self.opaque_files
            .iter()
            .filter_map(|path| self.nodes.get(path))
            .map(|node| (*node, OPAQUE_IMPORTER))
    }

    fn test_nodes(&self) -> HashMap<NodeIndex, PathBuf> {
        self.tests
            .keys()
            .filter_map(|path| self.nodes.get(path).map(|node| (*node, path.clone())))
            .collect()
    }
}

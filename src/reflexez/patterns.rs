use globset::Glob;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn matching_nodes(
    importer: &Path,
    pattern: &str,
    nodes: &HashMap<PathBuf, NodeIndex>,
    root: &Path,
) -> Vec<NodeIndex> {
    let matchers: Vec<_> = path_patterns(importer, pattern, root)
        .iter()
        .filter_map(|pattern| Glob::new(pattern).ok().map(|glob| glob.compile_matcher()))
        .collect();
    nodes
        .iter()
        .filter(|(path, _)| {
            let normalized = normalize(path);
            matchers.iter().any(|matcher| matcher.is_match(&normalized))
        })
        .map(|(_, index)| *index)
        .collect()
}

fn path_patterns(importer: &Path, pattern: &str, root: &Path) -> Vec<String> {
    if pattern.starts_with('.') {
        let parent = importer.parent().unwrap_or(root);
        return vec![normalize(&parent.join(pattern))];
    }
    if pattern.contains('/') {
        return vec![normalize(&root.join(pattern.trim_start_matches('/')))];
    }
    let module = pattern.replace('.', "/");
    let base = format!("{}/**/{module}", normalize(root));
    vec![format!("{base}.py"), format!("{base}/__init__.py")]
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

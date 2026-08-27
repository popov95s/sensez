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
        return vec![collapse_dot_components(&normalize(&parent.join(pattern)))];
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

fn collapse_dot_components(pattern: &str) -> String {
    let normalized = pattern.replace('\\', "/");
    let absolute = normalized.starts_with('/');
    let mut segments: Vec<String> = Vec::new();
    for segment in normalized.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if !absolute && segments.last().is_none_or(|last| last == "..") {
                    segments.push(segment.to_string());
                } else {
                    segments.pop();
                }
            }
            other => segments.push(other.to_string()),
        }
    }
    let joined = segments.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_patterns_lose_dot_components() {
        let root = Path::new("/repo");
        let importer = Path::new("/repo/src/routes/index.ts");

        let patterns = path_patterns(importer, "./pages/*.ts", root);
        assert_eq!(patterns, vec!["/repo/src/routes/pages/*.ts".to_string()]);

        let nested = path_patterns(importer, "./../shared/*.ts", root);
        assert!(nested[0].starts_with("/repo/src/shared/"));
    }
}

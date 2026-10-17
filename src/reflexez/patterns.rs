use globset::{Glob, GlobMatcher};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Nodes one glob call matches; `!`-prefixed patterns exclude.
pub fn matching_nodes(
    importer: &Path,
    patterns: &[String],
    nodes: &HashMap<PathBuf, NodeIndex>,
    root: &Path,
    resolve_alias: &mut impl FnMut(&str) -> Option<PathBuf>,
) -> Vec<NodeIndex> {
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for pattern in patterns {
        match pattern.strip_prefix('!') {
            Some(negated) => negative.extend(matchers(importer, negated, root, resolve_alias)),
            None => positive.extend(matchers(importer, pattern, root, resolve_alias)),
        }
    }
    nodes
        .iter()
        .filter(|(path, _)| {
            let normalized = normalize(path);
            positive.iter().any(|matcher| matcher.is_match(&normalized))
                && !negative.iter().any(|matcher| matcher.is_match(&normalized))
        })
        .map(|(_, index)| *index)
        .collect()
}

fn matchers(
    importer: &Path,
    pattern: &str,
    root: &Path,
    resolve_alias: &mut impl FnMut(&str) -> Option<PathBuf>,
) -> Vec<GlobMatcher> {
    path_patterns(importer, pattern, root, resolve_alias)
        .iter()
        .filter_map(|pattern| Glob::new(pattern).ok().map(|glob| glob.compile_matcher()))
        .collect()
}

fn path_patterns(
    importer: &Path,
    pattern: &str,
    root: &Path,
    resolve_alias: &mut impl FnMut(&str) -> Option<PathBuf>,
) -> Vec<String> {
    if pattern.starts_with('.') {
        let parent = importer.parent().unwrap_or(root);
        return vec![collapse_dot_components(&normalize(&parent.join(pattern)))];
    }
    if let Some(stripped) = pattern.strip_prefix('/') {
        return vec![normalize(&root.join(stripped))];
    }
    if let Some(aliased) = resolve_alias(pattern) {
        return vec![collapse_dot_components(&normalize(&aliased))];
    }
    if pattern.contains('/') {
        return vec![normalize(&root.join(pattern))];
    }
    let module = pattern.replace('.', "/");
    let base = format!("{}/**/{module}", normalize(root));
    vec![format!("{base}.py"), format!("{base}/__init__.py")]
}

/// `path` with `.` and `..` components folded away, without touching the filesystem.
pub(super) fn normalize_path(path: &Path) -> PathBuf {
    PathBuf::from(collapse_dot_components(&normalize(path)))
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

        let patterns = path_patterns(importer, "./pages/*.ts", root, &mut |_| None);
        assert_eq!(patterns, vec!["/repo/src/routes/pages/*.ts".to_string()]);

        let nested = path_patterns(importer, "./../shared/*.ts", root, &mut |_| None);
        assert!(nested[0].starts_with("/repo/src/shared/"));
    }

    #[test]
    fn alias_patterns_use_resolved_path() {
        let root = Path::new("/repo");
        let importer = Path::new("/repo/src/registry.ts");
        let mut alias = |spec: &str| {
            spec.strip_prefix("@/")
                .map(|rest| PathBuf::from("/repo/src").join(rest))
        };

        let patterns = path_patterns(importer, "@/handlers/*.ts", root, &mut alias);

        assert_eq!(patterns, vec!["/repo/src/handlers/*.ts".to_string()]);
    }
}

use crate::spine::graph::ModuleNode;
use globset::GlobSet;
use std::path::Path;

/// A compiled endpoint pattern: dotted-prefix (plain) or glob (name + path).
pub(super) enum Matcher {
    Prefix(String),
    Glob(GlobSet),
}

pub(super) fn compile(pattern: &str) -> Matcher {
    if pattern.contains(['*', '?', '[']) {
        Matcher::Glob(crate::globs::compile_validated(&[pattern]))
    } else {
        Matcher::Prefix(pattern.to_string())
    }
}

/// A node matches on its module name or, for globs, its file path.
pub(super) fn node_matches(matcher: &Matcher, node: &ModuleNode) -> bool {
    if matches_name(matcher, &node.module_name) {
        return true;
    }
    matches!(matcher, Matcher::Glob(set) if set.is_match(&node.file_path))
}

/// Prefix match is equal or a `.`/`/` submodule; globs also try the name with dots as slashes.
pub(super) fn matches_name(matcher: &Matcher, name: &str) -> bool {
    match matcher {
        Matcher::Prefix(prefix) => name
            .strip_prefix(prefix.as_str())
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '/'])),
        Matcher::Glob(set) => {
            set.is_match(Path::new(name)) || set.is_match(Path::new(&name.replace('.', "/")))
        }
    }
}

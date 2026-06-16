//! Aggregates parsed files into a directed graph where nodes are modules and
//! edges are import relationships (carrying the full [`ImportContext`]).

mod builder;

pub use builder::build;

use crate::profiles::Language;
use crate::spine::parser::ImportContext;
use crate::spine::parser::SymbolKind;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use std::path::PathBuf;

/// A module in the codebase (or a synthetic external dependency).
#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub file_path: PathBuf,
    pub module_name: String,
    /// The language of the source file (drives per-node dead-code dispatch).
    /// External/synthetic nodes inherit the importing module's language.
    pub language: Language,
    pub declared_public_symbols: Vec<String>,
    /// Top-level name → kind ("function" | "class" | "variable").
    pub declared_kinds: HashMap<String, SymbolKind>,
    /// Top-level name → 1-indexed definition line.
    pub declared_lines: HashMap<String, usize>,
    /// Module-level `__all__`, if declared (roots that are never dead).
    pub dunder_all: Option<Vec<String>>,
    /// Top-level symbol name → decorator trailing names.
    pub decorators: HashMap<String, Vec<String>>,
    /// Count of every identifier occurrence within the module's own source.
    pub name_counts: HashMap<String, usize>,
    /// True for stdlib/third-party/unresolved targets outside the scan tree.
    pub is_external: bool,
}

/// Two files claimed the same logical module identity.
#[derive(Debug, Clone)]
pub struct DuplicateModule {
    pub module_name: String,
    pub first_file: PathBuf,
    pub duplicate_file: PathBuf,
}

/// The directed import graph plus a module-name index.
#[derive(Default)]
pub struct CodebaseGraph {
    pub graph: DiGraph<ModuleNode, ImportContext>,
    pub name_to_index: HashMap<String, NodeIndex>,
    pub duplicate_modules: Vec<DuplicateModule>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::parser::parse_file;
    use std::fs;
    use std::path::Path;

    fn write(dir: &Path, rel: &str, body: &str) -> PathBuf {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn builds_nodes_and_resolves_edges() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let p_init = write(&dir, "app/__init__.py", "");
        let p_models = write(&dir, "app/models.py", "class User:\n    pass\n");
        let p_views = write(
            &dir,
            "app/views.py",
            "from app.models import User\nimport os\n\ndef show():\n    return User\n",
        );

        let files: Vec<_> = [&p_init, &p_models, &p_views]
            .iter()
            .enumerate()
            .map(|(i, p)| parse_file(p, i as u32).unwrap())
            .collect();

        let cg = build(&files, &[]);
        assert!(cg.name_to_index.contains_key("app.models"));
        assert!(cg.name_to_index.contains_key("app.views"));
        // `os` is unresolved -> synthetic external node.
        let os_idx = cg.name_to_index["os"];
        assert!(cg.graph[os_idx].is_external);
        // Edge app.views -> app.models carries imported symbol "User".
        let views = cg.name_to_index["app.views"];
        let models = cg.name_to_index["app.models"];
        let edge = cg.graph.find_edge(views, models).unwrap();
        assert_eq!(cg.graph[edge].imported_symbols, vec!["User"]);
    }

    /// Relative imports resolve against the importer's package: `from . import b`
    /// and `from .b import x` in `pkg.a` both target `pkg.b`.
    #[test]
    fn relative_imports_resolve_to_sibling() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let init = write(&dir, "pkg/__init__.py", "");
        let a = write(&dir, "pkg/a.py", "from . import b\nfrom .b import helper\n");
        let b = write(&dir, "pkg/b.py", "def helper():\n    return 1\n");

        let files: Vec<_> = [&init, &a, &b]
            .iter()
            .enumerate()
            .map(|(i, p)| parse_file(p, i as u32).unwrap())
            .collect();
        let cg = build(&files, &[]);

        let a_idx = cg.name_to_index["pkg.a"];
        let b_idx = cg.name_to_index["pkg.b"];
        assert!(
            cg.graph.find_edge(a_idx, b_idx).is_some(),
            "relative import pkg.a -> pkg.b must resolve"
        );
        // `from .b import helper` credits the symbol on (one of) the edges to pkg.b.
        let has_helper = cg
            .graph
            .edges_connecting(a_idx, b_idx)
            .any(|e| e.weight().imported_symbols.contains(&"helper".to_string()));
        assert!(has_helper, "from .b import helper must credit the symbol");
    }

    #[test]
    fn duplicate_module_names_are_recorded_not_silently_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let flat = write(&dir, "app.py", "def flat():\n    return 1\n");
        let init = write(&dir, "app/__init__.py", "def pkg():\n    return 2\n");

        let files: Vec<_> = [&flat, &init]
            .iter()
            .enumerate()
            .map(|(i, p)| parse_file(p, i as u32).unwrap())
            .collect();
        let cg = build(&files, &[]);

        assert_eq!(cg.duplicate_modules.len(), 1);
        assert_eq!(cg.duplicate_modules[0].module_name, "app");
        assert_eq!(cg.name_to_index.len(), 1, "duplicate must not overwrite");
    }
}

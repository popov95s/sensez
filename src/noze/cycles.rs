//! Pillar 3: circular-import detection via Tarjan's SCC algorithm.

use crate::globs::build_globset;
use crate::noze::{ActionLevel, CycleEdge, CycleFinding};
use crate::spine::graph::CodebaseGraph;
use petgraph::algo::tarjan_scc;
use petgraph::graph::NodeIndex;
use petgraph::visit::{EdgeFiltered, EdgeRef};
use petgraph::Direction;
use std::collections::HashMap;

/// Find every strongly connected component of cardinality >= 2.
///
/// Each such component is a set of modules that transitively import one
/// another at *module load time* — a real circular dependency. Inline
/// (function-local) imports are excluded, because they are precisely how
/// Python breaks import cycles and do not execute at load time; counting them
/// would report cycles that never actually occur. Module-hierarchy
/// declarations (Rust `mod x;`) are excluded too: containment is not coupling,
/// and parent-declares-child + child-uses-`super::` is the idiomatic layout.
/// For each cycle we also record one import edge per module (its next hop
/// within the cycle) with the source file/line, so the report is clickable.
pub fn detect(cg: &CodebaseGraph, exclude: &[String]) -> Vec<CycleFinding> {
    let excluded = build_globset(exclude);
    let module_level = EdgeFiltered::from_fn(&cg.graph, |edge| {
        !edge.weight().is_inline && !edge.weight().is_module_decl
    });
    let sccs: Vec<Vec<NodeIndex>> = tarjan_scc(&module_level)
        .into_iter()
        .filter(|scc| scc.len() >= 2)
        .filter(|scc| is_reportable_scc(cg, scc, &excluded))
        .collect();

    // node -> index of the SCC it belongs to (only the >=2 ones).
    let mut scc_of: HashMap<NodeIndex, usize> = HashMap::new();
    for (i, scc) in sccs.iter().enumerate() {
        for &n in scc {
            scc_of.insert(n, i);
        }
    }

    sccs.iter()
        .enumerate()
        .map(|(i, scc)| CycleFinding {
            action: ActionLevel::Warning,
            modules: scc
                .iter()
                .map(|&n| cg.graph[n].module_name.clone())
                .collect(),
            edges: scc
                .iter()
                .filter_map(|&n| next_hop(cg, n, i, &scc_of))
                .collect(),
        })
        .collect()
}

fn is_reportable_scc(cg: &CodebaseGraph, scc: &[NodeIndex], excluded: &globset::GlobSet) -> bool {
    scc.iter()
        .any(|&n| !excluded.is_match(&cg.graph[n].file_path))
}

/// The first module-level import from `node` to another member of its cycle.
fn next_hop(
    cg: &CodebaseGraph,
    node: NodeIndex,
    scc: usize,
    scc_of: &HashMap<NodeIndex, usize>,
) -> Option<CycleEdge> {
    cg.graph
        .edges_directed(node, Direction::Outgoing)
        .filter(|e| !e.weight().is_inline)
        .find(|e| scc_of.get(&e.target()) == Some(&scc))
        .map(|e| CycleEdge {
            from_module: cg.graph[node].module_name.clone(),
            to_module: cg.graph[e.target()].module_name.clone(),
            file: cg.graph[node].file_path.clone(),
            line: e.weight().line,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::parser::parse_file;
    use std::fs;

    #[test]
    fn detects_two_module_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("a.py"),
            "from b import beta\n\ndef alpha():\n    return beta\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.py"),
            "from a import alpha\n\ndef beta():\n    return alpha\n",
        )
        .unwrap();

        let files: Vec<_> = ["a.py", "b.py"]
            .iter()
            .enumerate()
            .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
            .collect();
        let cg = crate::spine::graph::build(&files, &[]);
        let cycles = detect(&cg, &[]);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].modules.len(), 2);
        // Each module contributes a clickable next-hop edge (file + import line).
        assert_eq!(cycles[0].edges.len(), 2);
        assert!(cycles[0]
            .edges
            .iter()
            .all(|e| e.line == 1 && (e.file.ends_with("a.py") || e.file.ends_with("b.py"))));
    }

    /// Inline (function-local) imports are how Python *breaks* cycles, so a
    /// mutual dependency that exists only via inline imports is NOT a cycle.
    #[test]
    fn inline_imports_do_not_form_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("a.py"),
            "def alpha():\n    from b import beta\n    return beta\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.py"),
            "def beta():\n    from a import alpha\n    return alpha\n",
        )
        .unwrap();

        let files: Vec<_> = ["a.py", "b.py"]
            .iter()
            .enumerate()
            .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
            .collect();
        let cg = crate::spine::graph::build(&files, &[]);
        assert!(
            detect(&cg, &[]).is_empty(),
            "inline-only mutual imports must not be a load-time cycle"
        );
    }

    #[test]
    fn excluded_test_fixture_cycles_are_silent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("tests/input/cycle")).unwrap();
        fs::write(
            dir.join("tests/input/cycle/a.py"),
            "from tests.input.cycle.b import beta\n",
        )
        .unwrap();
        fs::write(
            dir.join("tests/input/cycle/b.py"),
            "from tests.input.cycle.a import alpha\n",
        )
        .unwrap();

        let files: Vec<_> = ["tests/input/cycle/a.py", "tests/input/cycle/b.py"]
            .iter()
            .enumerate()
            .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
            .collect();
        let cg = crate::spine::graph::build(&files, &[]);
        assert!(detect(&cg, &["**/tests/**".to_string()]).is_empty());
    }
}

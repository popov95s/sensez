//! Pillar 3: circular-import detection via Tarjan's SCC algorithm.

use crate::globs::build_globset;
use crate::noze::{ActionLevel, CycleEdge, CycleFinding};
use crate::spine::graph::CodebaseGraph;
use crate::spine::ir::ImportPhase;
use petgraph::algo::tarjan_scc;
use petgraph::graph::NodeIndex;
use petgraph::visit::{EdgeFiltered, EdgeRef};
use petgraph::Direction;
use std::collections::{HashMap, HashSet};

/// Find strongly connected module components and report one clickable edge per module.
pub fn detect(cg: &CodebaseGraph, exclude: &[String]) -> Vec<CycleFinding> {
    let excluded = build_globset(exclude).unwrap_or_else(|_| globset::GlobSet::empty());
    let runtime = EdgeFiltered::from_fn(&cg.graph, |edge| {
        edge.weight().phase == ImportPhase::Runtime
            && !edge.weight().is_inline
            && !edge.weight().is_module_decl
    });
    let runtime_sccs: Vec<Vec<NodeIndex>> = filter_reportable_sccs(
        cg,
        tarjan_scc(&runtime)
            .into_iter()
            .filter(|scc| scc.len() >= 2),
        &excluded,
    );
    let mut findings = findings_for(cg, &runtime_sccs, ActionLevel::Warning, None, false);
    let runtime_keys: HashSet<String> = runtime_sccs.iter().map(|scc| scc_key(cg, scc)).collect();

    let type_level = EdgeFiltered::from_fn(&cg.graph, |edge| {
        matches!(
            edge.weight().phase,
            ImportPhase::Runtime | ImportPhase::TypeOnly
        ) && !edge.weight().is_inline
            && !edge.weight().is_module_decl
    });
    let hint = "type-only cycle: may indicate type coupling and dependent abstractions".to_string();
    findings.extend(
        filter_reportable_sccs(
            cg,
            tarjan_scc(&type_level)
                .into_iter()
                .filter(|scc| scc.len() >= 2),
            &excluded,
        )
        .into_iter()
        .filter(|scc| !runtime_keys.contains(&scc_key(cg, scc)))
        .filter(|scc| has_type_only_internal_edge(cg, scc))
        .map(|scc| finding_for(cg, &scc, ActionLevel::Info, Some(hint.clone()), true)),
    );
    findings
}

fn filter_reportable_sccs(
    cg: &CodebaseGraph,
    sccs: impl Iterator<Item = Vec<NodeIndex>>,
    excluded: &globset::GlobSet,
) -> Vec<Vec<NodeIndex>> {
    sccs.filter(|scc| is_reportable_scc(cg, scc, excluded))
        .collect()
}

fn findings_for(
    cg: &CodebaseGraph,
    sccs: &[Vec<NodeIndex>],
    action: ActionLevel,
    hint: Option<String>,
    include_type_only: bool,
) -> Vec<CycleFinding> {
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
            action,
            modules: scc
                .iter()
                .map(|&n| cg.graph[n].module_name.clone())
                .collect(),
            hint: hint.clone(),
            edges: scc
                .iter()
                .filter_map(|&n| next_hop(cg, n, i, &scc_of, include_type_only))
                .collect(),
        })
        .collect()
}

fn finding_for(
    cg: &CodebaseGraph,
    scc: &[NodeIndex],
    action: ActionLevel,
    hint: Option<String>,
    include_type_only: bool,
) -> CycleFinding {
    findings_for(cg, &[scc.to_vec()], action, hint, include_type_only)
        .pop()
        .unwrap_or_else(|| CycleFinding {
            action,
            modules: Vec::new(),
            hint: None,
            edges: Vec::new(),
        })
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
    include_type_only: bool,
) -> Option<CycleEdge> {
    cg.graph
        .edges_directed(node, Direction::Outgoing)
        .filter(|e| {
            !e.weight().is_inline
                && (e.weight().phase == ImportPhase::Runtime
                    || (include_type_only && e.weight().phase == ImportPhase::TypeOnly))
        })
        .find(|e| scc_of.get(&e.target()) == Some(&scc))
        .map(|e| CycleEdge {
            from_module: cg.graph[node].module_name.clone(),
            to_module: cg.graph[e.target()].module_name.clone(),
            file: cg.graph[node].file_path.clone(),
            line: e.weight().line,
        })
}

fn has_type_only_internal_edge(cg: &CodebaseGraph, scc: &[NodeIndex]) -> bool {
    let members: HashSet<NodeIndex> = scc.iter().copied().collect();
    scc.iter().any(|&node| {
        cg.graph
            .edges_directed(node, Direction::Outgoing)
            .any(|edge| {
                members.contains(&edge.target())
                    && edge.weight().phase == ImportPhase::TypeOnly
                    && !edge.weight().is_inline
                    && !edge.weight().is_module_decl
            })
    })
}

fn scc_key(cg: &CodebaseGraph, scc: &[NodeIndex]) -> String {
    let mut modules: Vec<&str> = scc
        .iter()
        .map(|&n| cg.graph[n].module_name.as_str())
        .collect();
    modules.sort_unstable();
    modules.join("|")
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
    fn type_checking_imports_form_info_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("a.py"),
            "import typing as t\n\nif t.TYPE_CHECKING:\n    from b import B\n\nclass A:\n    pass\n",
        )
        .unwrap();
        fs::write(dir.join("b.py"), "from a import A\n\nclass B:\n    pass\n").unwrap();

        let files: Vec<_> = ["a.py", "b.py"]
            .iter()
            .enumerate()
            .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
            .collect();
        let cg = crate::spine::graph::build(&files, &[]);
        let cycles = detect(&cg, &[]);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].action, ActionLevel::Info);
        assert!(cycles[0]
            .hint
            .as_deref()
            .is_some_and(|hint| hint.contains("type coupling")));
        assert!(cycles[0]
            .edges
            .iter()
            .any(|edge| { edge.from_module == "a" && edge.to_module == "b" && edge.line == 4 }));
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

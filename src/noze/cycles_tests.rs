use super::*;
use crate::spine::parser::parse_file;
use crate::test_support::write;

fn graph_for(sources: &[(&str, &str)]) -> CodebaseGraph {
    let tmp = tempfile::tempdir().unwrap();
    let files: Vec<_> = sources
        .iter()
        .enumerate()
        .map(|(i, (name, body))| parse_file(&write(tmp.path(), name, body), i as u32).unwrap())
        .collect();
    crate::spine::graph::build(&files, &[])
}

#[test]
fn detects_two_module_cycle() {
    let cg = graph_for(&[
        (
            "a.py",
            "from b import beta

def alpha():
    return beta
",
        ),
        (
            "b.py",
            "from a import alpha

def beta():
    return alpha
",
        ),
    ]);
    let cycles = detect(&cg, &Cycles::default());
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].modules.len(), 2);
    assert_eq!(cycles[0].edges.len(), 2);
    assert!(cycles[0]
        .edges
        .iter()
        .all(|e| e.line == 1 && (e.file.ends_with("a.py") || e.file.ends_with("b.py"))));
}

#[test]
fn edges_form_one_closed_loop() {
    let cg = graph_for(&[
        (
            "a.py",
            "from b import b
",
        ),
        (
            "b.py",
            "from a import a
from c import c
",
        ),
        (
            "c.py",
            "from b import b
",
        ),
    ]);
    let cycles = detect(&cg, &Cycles::default());
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].modules.len(), 3);
    let hops: Vec<_> = cycles[0]
        .edges
        .iter()
        .map(|e| (e.from_module.as_str(), e.to_module.as_str()))
        .collect();
    assert_eq!(hops, vec![("a", "b"), ("b", "a")]);
}

#[test]
fn reported_edges_always_chain_and_close() {
    let mut seed = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move |bound: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % bound
    };
    for _ in 0..200 {
        let mut cg = CodebaseGraph::default();
        let nodes: Vec<NodeIndex> = (0..2 + next(8))
            .map(|i| synthetic_module(&mut cg, &format!("m{i}")))
            .collect();
        for line in 0..next(20) {
            let from = nodes[next(nodes.len() as u64) as usize];
            let to = nodes[next(nodes.len() as u64) as usize];
            let phase = [ImportPhase::Runtime, ImportPhase::TypeOnly][next(2) as usize];
            cg.graph
                .add_edge(from, to, synthetic_import(line as usize, phase));
        }
        for cycle in detect(&cg, &Cycles::default()) {
            let (first, last) = (&cycle.edges[0], &cycle.edges[cycle.edges.len() - 1]);
            assert_eq!(last.to_module, first.from_module);
            assert!(cycle
                .edges
                .windows(2)
                .all(|w| w[0].to_module == w[1].from_module));
            assert!(cycle
                .edges
                .iter()
                .all(|e| cycle.modules.contains(&e.from_module)));
        }
    }
}

#[test]
fn type_only_cycles_can_be_switched_off() {
    let mut cg = CodebaseGraph::default();
    for (from, to, phase) in [
        ("x", "y", ImportPhase::Runtime),
        ("p", "q", ImportPhase::TypeOnly),
    ] {
        let (a, b) = (
            synthetic_module(&mut cg, from),
            synthetic_module(&mut cg, to),
        );
        cg.graph.add_edge(a, b, synthetic_import(1, phase));
        cg.graph.add_edge(b, a, synthetic_import(1, phase));
    }
    let off = Cycles {
        include_type_only: false,
        ..Cycles::default()
    };

    let modules = |config: &Cycles| -> Vec<Vec<String>> {
        let mut found: Vec<_> = detect(&cg, config)
            .into_iter()
            .map(|cycle| {
                let mut modules = cycle.modules;
                modules.sort();
                modules
            })
            .collect();
        found.sort();
        found
    };

    assert_eq!(modules(&off), vec![vec!["x", "y"]]);
    assert_eq!(
        modules(&Cycles::default()),
        vec![vec!["p", "q"], vec!["x", "y"]]
    );
}

fn synthetic_module(cg: &mut CodebaseGraph, name: &str) -> NodeIndex {
    let idx = cg.graph.add_node(crate::spine::graph::ModuleNode {
        file_path: format!("{name}.py").into(),
        module_name: name.to_string(),
        language: crate::spine::ir::Language::Python,
        source_index: None,
        is_external: false,
    });
    cg.name_to_index.insert(name.to_string(), idx);
    idx
}

fn synthetic_import(line: usize, phase: ImportPhase) -> ImportContext {
    ImportContext {
        source_module: String::new(),
        target_module: String::new(),
        imported_symbols: Vec::new(),
        bindings: Vec::new(),
        binding_phases: Vec::new(),
        line,
        column: 1,
        phase,
        is_inline: false,
        is_module_decl: false,
        is_reexport: false,
        enclosing_scope: None,
    }
}

#[test]
fn inline_imports_do_not_form_cycle() {
    let cg = graph_for(&[
        (
            "a.py",
            "def alpha():
    from b import beta
    return beta
",
        ),
        (
            "b.py",
            "def beta():
    from a import alpha
    return alpha
",
        ),
    ]);
    assert!(
        detect(&cg, &Cycles::default()).is_empty(),
        "inline-only mutual imports must not be a load-time cycle"
    );
}

#[test]
fn type_checking_imports_form_info_cycle() {
    let cg = graph_for(&[
        (
            "a.py",
            "import typing as t

if t.TYPE_CHECKING:
    from b import B

class A:
    pass
",
        ),
        (
            "b.py",
            "from a import A

class B:
    pass
",
        ),
    ]);
    let cycles = detect(&cg, &Cycles::default());
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
    let cg = graph_for(&[
        (
            "tests/input/cycle/a.py",
            "from tests.input.cycle.b import beta
",
        ),
        (
            "tests/input/cycle/b.py",
            "from tests.input.cycle.a import alpha
",
        ),
    ]);
    let config = Cycles {
        exclude: vec!["**/tests/**".to_string()],
        ..Cycles::default()
    };
    assert!(detect(&cg, &config).is_empty());
}

#[test]
fn self_import_does_not_shortcut_a_multi_module_loop() {
    let mut cg = CodebaseGraph::default();
    let (a, b) = (
        synthetic_module(&mut cg, "a"),
        synthetic_module(&mut cg, "b"),
    );
    cg.graph
        .add_edge(a, a, synthetic_import(1, ImportPhase::Runtime));
    cg.graph
        .add_edge(a, b, synthetic_import(2, ImportPhase::Runtime));
    cg.graph
        .add_edge(b, a, synthetic_import(3, ImportPhase::Runtime));
    let hops: Vec<_> = detect(&cg, &Cycles::default())[0]
        .edges
        .iter()
        .map(|e| (e.from_module.clone(), e.to_module.clone()))
        .collect();
    assert_eq!(hops, [("a".into(), "b".into()), ("b".into(), "a".into())]);
}

use super::*;
use crate::config::smells::Smells;
use crate::spine::parser::parse_file;
use std::fs;
use std::path::{Path, PathBuf};

fn parsed(name: &str, body: &str) -> crate::spine::parser::ParsedFile {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(format!("{name}.py"));
    fs::write(&path, body).unwrap();
    parse_file(&path, 0).unwrap()
}

fn local(name: &str, body: &str, cfg: &Smells) -> Vec<SmellFinding> {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(format!("{name}.py"));
    fs::write(&path, body).unwrap();
    let file = parse_file(&path, 0).unwrap();
    detect_local(&file, cfg)
}

fn kinds(findings: &[SmellFinding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.kind.as_str())
        .collect()
}

fn has(findings: &[SmellFinding], kind: &str) -> bool {
    findings.iter().any(|finding| finding.kind.as_str() == kind)
}

#[test]
fn god_module_is_a_hub_not_an_entrypoint() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    write(&dir, "pkg/__init__.py", "");
    // hub imports 4 leaves AND is imported by 4 dependents -> a real hub.
    write(
        &dir,
        "pkg/hub.py",
        "from pkg.l0 import a\nfrom pkg.l1 import a\nfrom pkg.l2 import a\nfrom pkg.l3 import a\n",
    );
    for n in 0..4 {
        write(&dir, &format!("pkg/l{n}.py"), "a = 1\n");
        write(&dir, &format!("pkg/d{n}.py"), "from pkg.hub import x\n");
    }
    // entry imports 8 modules but nobody imports it -> a composition root.
    write(
        &dir,
        "pkg/entry.py",
        "from pkg.l0 import a\nfrom pkg.l1 import a\nfrom pkg.l2 import a\nfrom pkg.l3 import a\nfrom pkg.d0 import x\nfrom pkg.d1 import x\nfrom pkg.d2 import x\nfrom pkg.d3 import x\n",
    );

    let paths = [
        "pkg/hub.py",
        "pkg/l0.py",
        "pkg/l1.py",
        "pkg/l2.py",
        "pkg/l3.py",
        "pkg/d0.py",
        "pkg/d1.py",
        "pkg/d2.py",
        "pkg/d3.py",
        "pkg/entry.py",
        "pkg/__init__.py",
    ];
    let parsed: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| parse_file(&dir.join(p), i as u32).unwrap())
        .collect();
    let graph = crate::spine::graph::build(&parsed, &[]);
    // Raise the shotgun threshold so the hub is evaluated as a god module rather
    // than being claimed first by the shotgun-hazard check.
    let cfg = Smells {
        god_module_fan: 8,
        shotgun_blast_threshold: 100,
        ..Smells::default()
    };
    let gm: Vec<String> = graphy::detect(&graph, &cfg.clone().into())
        .into_iter()
        .filter(|s| s.kind == SmellKind::GodModule)
        .map(|s| s.symbol)
        .collect();
    assert!(
        gm.iter().any(|m| m.ends_with("hub")),
        "the hub must be flagged: {gm:?}"
    );
    assert!(
        !gm.iter().any(|m| m.ends_with("entry")),
        "a pure entrypoint (fan-in 0) must not be a god module: {gm:?}"
    );
}

#[test]
fn primitive_params_alone_are_not_a_smell() {
    // Plain str/int/float params are idiomatic and must stay silent.
    let cfg = Smells::default();
    let body = "def f(a: str, b: int, c: float):\n    return a\n";
    assert!(local("po_off", body, &cfg).is_empty());
}

#[test]
fn linter_owned_smells_are_off_by_default() {
    let cfg = Smells::default();
    // Many params, magic number, 6 returns — linter-owned, so silent here.
    let body = "def f(a, b, c, d, e, f, g):\n    x = 9999\n    if a: return 1\n    if b: return 2\n    if c: return 3\n    if d: return 4\n    if e: return 5\n    return x\n";
    let found = local("linter_off", body, &cfg);
    for kind in [
        "magic_numbers",
        "long_parameter_list",
        "too_many_returns",
        "high_complexity",
    ] {
        assert!(
            !has(&found, kind),
            "{kind} should be off by default (language linters cover it)"
        );
    }
}

#[test]
fn data_clumps_need_min_support() {
    let cfg = Smells::default(); // min_fields = 4, min_occurrences = 3
    let sig = "(start, end, tz, fmt)";
    let two = format!("def a{sig}:\n    return 1\ndef b{sig}:\n    return 2\n");
    let pf = parsed("clump2", &two);
    assert!(
        !has(&clumps::detect(&[&pf], &cfg.clone().into()), "data_clump"),
        "2 < 3 occurrences"
    );
    let three = format!("{two}def c{sig}:\n    return 3\n");
    let pf3 = parsed("clump3", &three);
    assert!(has(
        &clumps::detect(&[&pf3], &cfg.clone().into()),
        "data_clump"
    ));
}

#[test]
fn data_clump_respects_min_fields_default() {
    let cfg = Smells::default(); // min_fields = 4
                                 // A recurring 3-field bundle must NOT trigger at the default minimum of 4.
    let body = "def a(x, y, z):\n    return 1\ndef b(x, y, z):\n    return 2\ndef c(x, y, z):\n    return 3\n";
    let pf = parsed("clump_3field", body);
    assert!(
        !has(&clumps::detect(&[&pf], &cfg.clone().into()), "data_clump"),
        "3-field bundle is below the default min_fields of 4"
    );
}

fn write(dir: &Path, rel: &str, body: &str) -> PathBuf {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, body).unwrap();
    path
}

#[test]
fn shotgun_hazard_needs_blast_radius() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let core = write(&dir, "pkg/core.py", "def helper():\n    return 1\n");
    let mut files = vec![core];
    // Five distinct dependents import pkg.core -> blast = 5 >= threshold 4.
    for i in 0..5 {
        files.push(write(
            &dir,
            &format!("pkg/m{i}.py"),
            "from pkg.core import helper\n",
        ));
    }
    write(&dir, "pkg/__init__.py", "");
    let parsed: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(i, p)| parse_file(p, i as u32).unwrap())
        .collect();
    let graph = crate::spine::graph::build(&parsed, &[]);
    let cfg = Smells::default();
    let findings = graphy::detect(&graph, &cfg.clone().into());
    assert!(
        has(&findings, "shotgun_surgery_hazard"),
        "kinds: {:?}",
        kinds(&findings)
    );
}

#[test]
fn package_index_barrels_are_not_graph_hotspots() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let barrel = write(&dir, "pkg/__init__.py", "from pkg.core import helper\n");
    write(&dir, "pkg/core.py", "def helper():\n    return 1\n");
    let mut files = vec![barrel];
    for i in 0..5 {
        files.push(write(&dir, &format!("pkg/m{i}.py"), "import pkg\n"));
    }
    let parsed: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(i, p)| parse_file(p, i as u32).unwrap())
        .collect();
    let graph = crate::spine::graph::build(&parsed, &[]);
    let cfg = Smells::default();
    let findings = graphy::detect(&graph, &cfg.clone().into());
    assert!(
        findings
            .iter()
            .all(|f| !f.file.ends_with("pkg/__init__.py")),
        "package API barrels should not be graph smell hotspots: {findings:?}"
    );
}

#[test]
fn split_variable_flags_branch_bound_local_at_min_two() {
    // The motivating case: one name bound in both arms of an if/else. At
    // min_assigns = 2 this enforces single-binding locals.
    let cfg = Smells {
        split_variable: true,
        split_variable_min_assigns: 2,
        ..Smells::default()
    };
    let body = "def f(x):\n    if x:\n        plan = make(x)\n    else:\n        plan = load()\n    return plan\n";
    let f = local("sv_branch", body, &cfg);
    assert!(has(&f, "split_variable"), "kinds: {:?}", kinds(&f));
}

#[test]
fn split_variable_default_min_flags_double_assignment() {
    // Default min_assigns is 2: enabling the smell catches branch-bound locals.
    let cfg = Smells {
        split_variable: true,
        ..Smells::default()
    };
    let body = "def f(x):\n    if x:\n        plan = make(x)\n    else:\n        plan = load()\n    return plan\n";
    assert!(has(&local("sv_default", body, &cfg), "split_variable"));
}

#[test]
fn split_variable_single_binding_is_silent() {
    let cfg = Smells {
        split_variable: true,
        split_variable_min_assigns: 2,
        ..Smells::default()
    };
    let body = "def f(x):\n    plan = make(x)\n    return plan\n";
    assert!(!has(&local("sv_single", body, &cfg), "split_variable"));
}

#[test]
fn disabled_pillar_is_silent() {
    let cfg = Smells {
        enabled: false,
        ..Smells::default()
    };
    let pf = parsed("disabled", "def f(a,b,c,d,e,f,g):\n    return 1\n");
    assert!(detect(&[pf], &CodebaseGraph::default(), &cfg.clone().into()).is_empty());
}

use crate::spine::graph::CodebaseGraph;

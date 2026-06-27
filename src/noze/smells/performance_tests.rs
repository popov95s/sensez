use super::detect_local;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::parser::parse_file;
use std::fs;

fn local(ext: &str, body: &str) -> Vec<SmellFinding> {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(format!("sample.{ext}"));
    fs::write(&path, body).unwrap();
    let file = parse_file(&path, 0).unwrap();
    detect_local(&file, &Smells::default())
}

fn has(findings: &[SmellFinding], kind: SmellKind) -> bool {
    findings.iter().any(|f| f.kind == kind)
}

#[test]
fn python_flags_direct_performance_shapes() {
    let src = "\
def f(db, ids, xs):
    for id in ids:
        rows = []
        for x in xs:
            rows.append(x)
        rows.sort()
        db.get(id)
    if any(xs):
        return sum(xs)
";
    let findings = local("py", src);
    assert!(has(&findings, SmellKind::NestedLoop), "{findings:?}");
    assert!(has(&findings, SmellKind::SortInLoop), "{findings:?}");
    assert!(has(&findings, SmellKind::NPlusOneCall), "{findings:?}");
    assert!(has(&findings, SmellKind::RepeatedIteration), "{findings:?}");
}

#[test]
fn js_flags_direct_performance_shapes() {
    let src = "\
export function f(api, ids, items) {
  for (const id of ids) {
    for (const item of items) api.touch(item);
    items.sort();
    api.fetch(id);
  }
  const found = items.some(Boolean);
  return found || items.filter(Boolean).length > 0;
}
";
    let findings = local("js", src);
    assert!(has(&findings, SmellKind::NestedLoop), "{findings:?}");
    assert!(has(&findings, SmellKind::SortInLoop), "{findings:?}");
    assert!(has(&findings, SmellKind::NPlusOneCall), "{findings:?}");
    assert!(has(&findings, SmellKind::RepeatedIteration), "{findings:?}");
}

#[test]
fn loop_calling_helper_that_loops_is_flagged() {
    let src = "\
def helper(values):
    for value in values:
        print(value)

def f(groups):
    for group in groups:
        helper(group)
";
    let findings = local("py", src);
    assert!(has(&findings, SmellKind::NestedLoop), "{findings:?}");
}

#[test]
fn recursive_tree_walk_is_not_helper_nested_loop() {
    let src = "\
fn visit(items: &[u32]) {
    for item in items {
        if *item > 0 {
            visit(items);
        }
    }
}
";
    let findings = local("rs", src);
    assert!(!has(&findings, SmellKind::NestedLoop), "{findings:?}");
}

#[test]
fn loop_calling_helper_with_external_call_is_n_plus_one() {
    let src = "\
function load(client, id) {
  return client.fetch(id);
}

export function f(client, ids) {
  for (const id of ids) {
    load(client, id);
  }
}
";
    let findings = local("js", src);
    assert!(has(&findings, SmellKind::NPlusOneCall), "{findings:?}");
}

#[test]
fn dict_get_over_fixed_keys_is_not_n_plus_one() {
    let src = "\
def json_finding_count(data):
    if isinstance(data, dict):
        for k in (\"total_issues\", \"findings\", \"issues\", \"results\"):
            v = data.get(k)
            if isinstance(v, int):
                return v
";
    let findings = local("py", src);
    assert!(!has(&findings, SmellKind::NPlusOneCall), "{findings:?}");
}

#[test]
fn get_receiver_heuristics_are_language_specific() {
    let py = "\
def f(api, ids):
    for id in ids:
        api.get(id)
";
    assert!(!has(&local("py", py), SmellKind::NPlusOneCall));

    let js = "\
export function f(api, ids) {
  for (const id of ids) {
    api.get(id);
  }
}
";
    assert!(has(&local("js", js), SmellKind::NPlusOneCall));
}

#[test]
fn nested_loop_over_config_constant_is_not_flagged() {
    let src = "\
SOLUTIONS = []

def build_rows(order):
    rows = []
    for target in order:
        comps = []
        for comp in SOLUTIONS:
            comps.append(comp.name)
        rows.append((target, comps))
    return rows
";
    let findings = local("py", src);
    assert!(!has(&findings, SmellKind::NestedLoop), "{findings:?}");
}

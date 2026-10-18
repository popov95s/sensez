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
    let mut cfg = Smells::default();
    cfg.disabled
        .retain(|kind| !matches!(kind, SmellKind::NestedLoop | SmellKind::NPlusOneCall));
    detect_local(&file, &cfg)
}

fn has(findings: &[SmellFinding], kind: SmellKind) -> bool {
    findings.iter().any(|f| f.kind == kind)
}

#[test]
fn nested_loop_and_n_plus_one_are_off_by_default() {
    let src = "\
def f(db, ids, xs):
    for id in ids:
        for x in xs:
            db.fetch(x)
";
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("sample.py");
    fs::write(&path, src).unwrap();
    let file = parse_file(&path, 0).unwrap();
    let findings = detect_local(&file, &Smells::default());
    assert!(!has(&findings, SmellKind::NestedLoop), "{findings:?}");
    assert!(!has(&findings, SmellKind::NPlusOneCall), "{findings:?}");
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
        db.fetch(id)
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
  const found = items.filter(Boolean);
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
fn plain_get_is_not_an_external_loop_call() {
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
    assert!(!has(&local("js", js), SmellKind::NPlusOneCall));

    let rs = "\
fn f(map: &std::collections::HashMap<u32, u32>, ids: &[u32]) {
    for id in ids {
        let _ = map.get(id);
    }
}
";
    assert!(!has(&local("rs", rs), SmellKind::NPlusOneCall));
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

#[test]
fn repeated_iteration_keeps_full_receiver_path() {
    let src = "\
def summarize(report):
    errors = report.errors.count(\"error\")
    warnings = report.warnings.count(\"warning\")
    return errors + warnings
";
    let findings = local("py", src);
    assert!(
        !has(&findings, SmellKind::RepeatedIteration),
        "{findings:?}"
    );
}

#[test]
fn mutation_between_iteration_passes_suppresses_finding() {
    let src = "\
def summarize(items):
    before = sum(items)
    items.append(3)
    return max(items)
";
    let findings = local("py", src);
    assert!(
        !has(&findings, SmellKind::RepeatedIteration),
        "{findings:?}"
    );
}

#[test]
fn mutation_on_the_same_line_as_the_passes_suppresses_finding() {
    let src = "export function f(xs: number[]) { const a = xs.map(g); xs.push(1); return xs.filter(h).length + a.length; }\n";
    let findings = local("ts", src);
    assert!(
        !has(&findings, SmellKind::RepeatedIteration),
        "{findings:?}"
    );
}

#[test]
fn mutually_exclusive_iteration_passes_are_not_combined() {
    let src = "\
def summarize(items, detailed):
    if detailed:
        return sum(items)
    else:
        return max(items)
";
    let findings = local("py", src);
    assert!(
        !has(&findings, SmellKind::RepeatedIteration),
        "{findings:?}"
    );
}

#[test]
fn js_repeated_iteration_false_positive_shapes_are_quiet() {
    for src in [
        "it(\"splits\", () => { expect(xs.filter(x => x.ok)).toHaveLength(3); expect(xs.filter(x => !x.ok)).toHaveLength(1); });\n",
        "export function check(xs: X[]) { expect(xs.filter(x => x.ok)).toHaveLength(3); assert.equal(xs.filter(x => !x.ok).length, 1); }\n",
        "export const mixed = (xs: X[]) => xs.some(x => !x.ok) && xs.every(x => x.n > 0);\n",
        "export function rel(xs: X[]) { const f = xs.find(x => x.ok); if (!f) return []; return xs.map(x => x.n - f.n); }\n",
        "export function early(xs: X[]) { const a = xs.filter(x => x.ok); if (!a.length) return []; return xs.map(x => x.n); }\n",
        "export async function keep(xs: X[]) { const k = await load(xs.map(x => x.id)); return xs.filter(x => k.includes(x.id)); }\n",
        "export const mid = (ps: P[]) => [median(ps.map(p => p.x)), median(ps.map(p => p.y))];\n",
        "export const box = (ps: P[]) => [max(ps.map((p) => p.x)), max(ps.map((p: P) => p.y))];\n",
    ] {
        assert!(!has(&local("ts", src), SmellKind::RepeatedIteration), "{src}");
    }
}

#[test]
fn js_repeated_iteration_true_positives_keep_firing() {
    for src in [
        "export function buckets(xs: X[]) { const a = xs.filter(x => x.k === \"a\"); const b = xs.filter(x => x.k === \"b\"); const c = xs.filter(x => x.k === \"c\"); return [a, b, c]; }\n",
        "export function totals(xs: X[]) { const n = xs.filter(x => x.ok).length; const s = xs.map(x => x.v).reduce((p, q) => p + q, 0); return [n, s]; }\n",
        "export function spread(xs: X[][]) { return [xs.flatMap(x => x), xs.flatMap(x => x.slice(1))]; }\n",
    ] {
        assert!(has(&local("ts", src), SmellKind::RepeatedIteration), "{src}");
    }
}

#[test]
fn repeated_iteration_message_lists_lines_and_methods() {
    let src = "export function buckets(xs: X[]) {\n  const a = xs.filter(x => x.k === \"a\");\n  const b = xs.filter(x => x.k === \"b\");\n  return [a, b];\n}\n";
    let finding = local("ts", src)
        .into_iter()
        .find(|f| f.kind == SmellKind::RepeatedIteration)
        .expect("partition must fire");
    assert_eq!(finding.line, 2);
    assert!(
        finding
            .message
            .contains("`xs` is iterated 2 times in this scope (lines 2, 3: filter)"),
        "{}",
        finding.message
    );
}

#[test]
fn include_tests_restores_assertion_passes() {
    let src = "it(\"splits\", () => { expect(xs.filter(x => x.ok)).toHaveLength(3); expect(xs.filter(x => !x.ok)).toHaveLength(1); });\n";
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("rows.test.ts");
    fs::write(&path, src).unwrap();
    let file = parse_file(&path, 0).unwrap();
    let cfg = Smells {
        repeated_iteration_include_tests: true,
        ..Smells::default()
    };
    assert!(has(
        &detect_local(&file, &cfg),
        SmellKind::RepeatedIteration
    ));
    assert!(!has(
        &detect_local(&file, &Smells::default()),
        SmellKind::RepeatedIteration
    ));
}

#[test]
fn small_literal_ranges_and_take_are_bounded() {
    let py = "\
def pairs(items):
    for item in items:
        for index in range(5):
            print(item, index)
";
    assert!(!has(&local("py", py), SmellKind::NestedLoop));

    let rust = "\
fn pairs(items: &[u32]) {
    for item in items {
        for index in (0..100).take(5) {
            println!(\"{item} {index}\");
        }
    }
}
";
    assert!(!has(&local("rs", rust), SmellKind::NestedLoop));
}

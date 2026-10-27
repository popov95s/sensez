use super::detect_local;
use super::test_support as fixture;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};

fn findings(ext: &str, source: &str, config: &Smells, kind: SmellKind) -> Vec<SmellFinding> {
    let file = fixture::parsed(&format!("sample.{ext}"), source);
    detect_local(&file, config)
        .into_iter()
        .filter(|finding| finding.kind == kind)
        .collect()
}

fn opt_in() -> Smells {
    Smells {
        split_variable: true,
        too_many_returns: true,
        magic_numbers: true,
        ..Smells::default()
    }
}

fn split(ext: &str, source: &str) -> Vec<SmellFinding> {
    findings(ext, source, &opt_in(), SmellKind::SplitVariable)
}

#[test]
fn split_variable_counts_one_assignment_per_exclusive_arm() {
    let chain = r#"
export function split(a, b) {
  let label;
  if (a) label = "x";
  else if (b) label = "y";
  else label = "z";
  return label;
}
"#;
    let found = split("ts", chain);
    assert!(found.is_empty(), "{found:?}");

    let switch = r#"
export function pick(kind) {
  let label;
  switch (kind) {
    case "a":
      label = "x";
      break;
    case "b":
      label = "y";
      break;
    default:
      label = "z";
  }
  return label;
}
"#;
    let found = split("ts", switch);
    assert!(found.is_empty(), "{found:?}");

    let python = r#"
def split(a, b):
    if a:
        label = "x"
    elif b:
        label = "y"
    else:
        label = "z"
    return label
"#;
    let found = split("py", python);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn split_variable_keeps_sequential_reassignment() {
    let sequential = r#"
export function reuse() {
  let s;
  s = a();
  use(s);
  s = b();
  use(s);
}
"#;
    let found = split("ts", sequential);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("assigned 2 times"),
        "{}",
        found[0].message
    );

    let then_branch = r#"
export function reset(a) {
  let x;
  x = start();
  if (a) x = 2;
  else x = 3;
  return x;
}
"#;
    assert!(split("ts", then_branch)[0]
        .message
        .contains("assigned 2 times"));

    let python = "def reuse():\n    s = a()\n    use(s)\n    s = b()\n    use(s)\n";
    assert_eq!(split("py", python).len(), 1);
}

fn returns(source: &str, config: &Smells) -> Vec<SmellFinding> {
    findings("ts", source, config, SmellKind::TooManyReturns)
}

#[test]
fn too_many_returns_skips_top_level_guard_clauses() {
    let guards = r#"
export function guards(a, b, c, d) {
  if (!a) return 1;
  if (!b) return 2;
  if (!c) {
    return 3;
  }
  if (!d) return 4;
  return 5;
}
"#;
    let found = returns(guards, &opt_in());
    assert!(found.is_empty(), "{found:?}");

    let counted = Smells {
        count_guard_clauses: true,
        ..opt_in()
    };
    assert_eq!(returns(guards, &counted).len(), 1);

    let python = "def guards(a, b, c, d):\n    if not a:\n        return 1\n    if not b:\n        return 2\n    if not c:\n        return 3\n    if not d:\n        return 4\n    return 5\n";
    let found = findings("py", python, &opt_in(), SmellKind::TooManyReturns);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn too_many_returns_keeps_branch_returns_and_reports_excluded_guards() {
    let ifs = r#"
export function ifs(x, y) {
  if (!y) return "none";
  if (x === 1) {
    return "a";
  } else if (x === 2) {
    return "b";
  } else if (x === 3) {
    return "c";
  } else if (x === 4) {
    return "d";
  } else {
    return "e";
  }
}
"#;
    let found = returns(ifs, &opt_in());
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].message,
        "5 return statements (threshold 4, 1 guard clause(s) excluded)"
    );
}

fn magic(ext: &str, source: &str, config: &Smells) -> Vec<SmellFinding> {
    findings(ext, source, config, SmellKind::MagicNumbers)
}

const CONTEXTS: &str = r#"
export function magic(res, n, list, retries = 5) {
  const TTL_MS = 5000;
  if (res.status === 404) return;
  res.statusCode = 503;
  res.status(201);
  const day = 24 * 60 * 60 * 1000;
  setTimeout(go, 500);
  const shown = n.toFixed(3).padStart(8);
  return { status: 422, day, shown, TTL_MS, third: list[3] };
}
"#;

#[test]
fn magic_numbers_skip_named_and_conventional_contexts() {
    let found = magic("ts", CONTEXTS, &opt_in());
    assert!(found.is_empty(), "{found:?}");

    let python = r#"
def magic(res, items, retries=5):
    TIMEOUT_S = 30
    if res.status_code == 404:
        return None
    time.sleep(45)
    day = 24 * 60 * 60
    return items[3], day, TIMEOUT_S
"#;
    let found = magic("py", python, &opt_in());
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn magic_number_knobs_restore_contexts() {
    let strict = Smells {
        magic_number_ignore_array_indexes: false,
        magic_number_ignore_default_values: false,
        magic_number_allow_http_status: false,
        ..opt_in()
    };
    let found = magic("ts", CONTEXTS, &strict);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].message,
        "6 magic numeric literal(s): 5@L2, 404@L4, 503@L5 (+3 more)"
    );
    assert_eq!(found[0].line, 2);
}

#[test]
fn magic_numbers_report_each_literal_site() {
    let source = r#"
export function price(retries, total) {
  if (retries > 7) retry();
  return total * 1.19;
}
"#;
    let found = magic("ts", source, &opt_in());
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].message,
        "2 magic numeric literal(s): 7@L3, 1.19@L4"
    );
    assert_eq!(found[0].line, 3);
    assert_eq!(found[0].lines, vec![3, 4]);

    let ignoring = Smells {
        magic_number_ignore: vec!["7".to_string(), "1.19".to_string()],
        ..opt_in()
    };
    let found = magic("ts", source, &ignoring);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn split_variable_findings_follow_local_name_order() {
    let source = "export function f(c) {\n  let e = 1; let d = 1; let a = 1; let c2 = 1; let b = 1;\n  if (c) { e = 2; d = 2; a = 2; c2 = 2; b = 2; }\n  e = 3; d = 3; a = 3; c2 = 3; b = 3;\n  return a + b + c2 + d + e;\n}\n";
    let names: Vec<String> = split("ts", source)
        .iter()
        .map(|finding| finding.message.split('`').nth(1).unwrap().to_string())
        .collect();
    assert_eq!(names, ["a", "b", "c2", "d", "e"], "{names:?}");
}

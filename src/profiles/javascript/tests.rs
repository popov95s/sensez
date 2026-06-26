//! JavaScript profile behavior tests (structural pillars: tokens, imports,
//! resolution, cycles, duplication, cross-language isolation).

use super::JsProfile;
use crate::spine::parser::{parse_file, parse_source, StructuralToken};
use std::fs;

fn tokens(src: &[u8]) -> Vec<StructuralToken> {
    parse_source(src, 0, "m", &JsProfile).unwrap().syntax.tokens
}

/// Two functions with the same control-flow shape but different local names
/// produce identical structural-token vectors (rename invariance).
#[test]
fn rename_invariance_yields_identical_tokens() {
    let a = b"function compute(items) {\n  let total = 0;\n  for (const item of items) {\n    if (item > 10) { total = total + item; }\n  }\n  return total;\n}\n";
    let b = b"function process(values) {\n  let acc = 99;\n  for (const value of values) {\n    if (value > 500) { acc = acc + value; }\n  }\n  return acc;\n}\n";
    let ta = tokens(a);
    assert_eq!(
        ta,
        tokens(b),
        "renamed-but-identical JS functions must match"
    );
    assert!(ta.contains(&StructuralToken::FunctionDef));
    assert!(ta.contains(&StructuralToken::ForStatement));
    assert!(ta.contains(&StructuralToken::IfStatement));
    assert!(ta.contains(&StructuralToken::Return));
}

/// `import`, re-export `from`, and CommonJS `require` are all extracted.
#[test]
fn extracts_es_and_commonjs_imports() {
    let src = b"import { a, b as c } from './mod';\nimport def from 'pkg';\nexport { x } from './re';\nconst fsx = require('fs');\n";
    let imports = parse_source(src, 0, "m", &JsProfile)
        .unwrap()
        .symbols
        .imports;
    let targets: Vec<&str> = imports.iter().map(|i| i.target_module.as_str()).collect();
    assert!(targets.contains(&"./mod"));
    assert!(targets.contains(&"pkg"));
    assert!(
        targets.contains(&"./re"),
        "re-export from is an import edge"
    );
    assert!(targets.contains(&"fs"), "require() is an import edge");

    let named = imports.iter().find(|i| i.target_module == "./mod").unwrap();
    assert_eq!(named.imported_symbols, vec!["a", "b"]);
    assert_eq!(named.bindings, vec!["a", "c"]); // alias-aware
}

/// Relative imports resolve to sibling module keys and a mutual import is a cycle.
#[test]
fn relative_imports_resolve_and_detect_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("package.json"), "{\"name\":\"x\"}\n").unwrap();
    fs::write(
        dir.join("src/a.js"),
        "import { b } from './b';\nexport function a() { return b(); }\n",
    )
    .unwrap();
    fs::write(
        dir.join("src/b.js"),
        "import { a } from './a';\nexport function b() { return a(); }\n",
    )
    .unwrap();

    let files: Vec<_> = ["src/a.js", "src/b.js"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    assert!(cg.name_to_index.contains_key("src/a"));
    assert!(cg.name_to_index.contains_key("src/b"));

    let cycles = crate::noze::cycles::detect(&cg, &[]);
    assert_eq!(cycles.len(), 1, "src/a <-> src/b is a circular import");
}

/// Two byte-identical JS functions in different files are a clone.
#[test]
fn identical_functions_are_a_clone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    let body = "export function handle(req, res) {\n  const data = req.body;\n  if (data.id > 0) {\n    res.send(data.name);\n  } else {\n    res.fail(data.code);\n  }\n  return data.id + data.code;\n}\n";
    fs::write(dir.join("one.js"), body).unwrap();
    fs::write(dir.join("two.js"), body).unwrap();

    let files: Vec<_> = ["one.js", "two.js"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cfg = crate::config::model::Duplication {
        exclude: vec![],
        threshold: 10,
        max_gap: 0,
        ..Default::default()
    };
    let dup = crate::noze::duplication::detect(&files, &cfg);
    assert!(!dup.is_empty(), "identical JS functions must be a clone");
}

/// A Python and a JS file with the same control-flow shape must NOT be reported
/// as a cross-language clone (duplication is partitioned per language).
#[cfg(feature = "lang-python")]
#[test]
fn no_cross_language_clones() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("m.py"),
        "def handle(req, res):\n    data = req\n    if data > 0:\n        res.send(data)\n    else:\n        res.fail(data)\n    return data + data\n",
    )
    .unwrap();
    fs::write(
        dir.join("m.js"),
        "function handle(req, res) {\n  let data = req;\n  if (data > 0) { res.send(data); } else { res.fail(data); }\n  return data + data;\n}\n",
    )
    .unwrap();

    let files: Vec<_> = ["m.py", "m.js"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cfg = crate::config::model::Duplication {
        exclude: vec![],
        threshold: 6,
        max_gap: 0,
        ..Default::default()
    };
    let dup = crate::noze::duplication::detect(&files, &cfg);
    assert!(
        dup.is_empty(),
        "Python and JS must never form a cross-language clone; got {dup:?}"
    );
}

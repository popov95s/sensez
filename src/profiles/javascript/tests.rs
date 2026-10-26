//! JavaScript profile behavior tests (structural pillars: tokens, imports,
//! resolution, cycles, duplication, cross-language isolation).

use super::JsProfile;
use crate::spine::parser::{parse_file, parse_source, StructuralToken};
use std::fs;

pub(super) fn tokens(src: &[u8]) -> Vec<StructuralToken> {
    parse_source(src, 0, "m", &JsProfile).unwrap().syntax.tokens
}

#[cfg(feature = "eyez")]
#[test]
fn jsdoc_before_declaration_belongs_to_function() {
    let walked = parse_source(
        b"/** Parse the incoming request into a verified user identity. */\nexport function authenticate() { return true; }\n// This describes the next function and its intent.\nfunction authorize() { return true; }\n",
        0,
        "m",
        &JsProfile,
    )
    .unwrap();
    assert_eq!(walked.docs[0].symbol_path, "m::authenticate");
    assert_eq!(walked.docs[1].symbol_path, "m::authorize");
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

#[test]
fn extracts_dynamic_import_destructuring_and_namespace_bindings() {
    let src =
        b"const { load: fetch, save } = await import('./store');\nconst api = require('./api');\n";
    let imports = parse_source(src, 0, "m", &JsProfile)
        .unwrap()
        .symbols
        .imports;

    let store = imports
        .iter()
        .find(|import| import.target_module == "./store")
        .unwrap();
    assert_eq!(store.imported_symbols, vec!["load", "save"]);
    assert_eq!(store.bindings, vec!["fetch", "save"]);

    let api = imports
        .iter()
        .find(|import| import.target_module == "./api")
        .unwrap();
    assert!(api.imported_symbols.is_empty());
    assert_eq!(api.bindings, vec!["api"]);
}

#[test]
fn credits_template_shorthand_and_type_references() {
    let src = b"const LABEL = 'ok';\nconst item = { LABEL };\nconst text = `${LABEL}`;\ntype Label = typeof LABEL;\n";
    let walked = parse_source(src, 0, "m", &JsProfile).unwrap();
    assert_eq!(walked.usage.name_counts.get("LABEL"), Some(&4));
}

#[test]
fn function_facts_preserve_order_and_exclude_nested_bodies() {
    let src = b"function outer(items) {\n  if (items.length) return make(items);\n  function inner(value) { return value + 42; }\n  return this.finish(items);\n}\n";
    let functions = parse_source(src, 0, "m", &JsProfile)
        .unwrap()
        .units
        .functions;

    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].name, "outer");
    assert_eq!(functions[1].name, "inner");
    assert!(!functions[0].is_nested);
    assert!(functions[1].is_nested);
    assert_eq!(functions[1].parent, "outer");
    assert_eq!(functions[0].return_count, 2);
    assert_eq!(functions[1].return_count, 1);
    assert_eq!(functions[0].magic_numbers.len(), 0);
    assert_eq!(functions[1].magic_numbers.len(), 1);
    assert_eq!(functions[0].branch_count, 1);
}

#[test]
fn else_if_chain_stays_at_one_nesting_level() {
    let src = b"function pick(n) {\n  if (n === 1) {\n    return 'a';\n  } else if (n === 2) {\n    return 'b';\n  } else if (n === 3) {\n    return 'c';\n  } else if (n === 4) {\n    return 'd';\n  } else if (n === 5) {\n    return 'e';\n  }\n  return 'z';\n}\n";
    let f = &parse_source(src, 0, "m", &JsProfile)
        .unwrap()
        .units
        .functions[0];
    assert_eq!(f.max_nesting, 1);
    assert_eq!(f.cognitive, 5);
}

#[test]
fn nested_blocks_still_count_every_level() {
    let src = b"function walk(xs) {\n  if (xs) {\n    for (const row of xs) {\n      if (row) {\n        while (row.length) {\n          if (row.pop()) {\n            return 1;\n          }\n        }\n      }\n    }\n  }\n  return 0;\n}\n";
    let f = &parse_source(src, 0, "m", &JsProfile)
        .unwrap()
        .units
        .functions[0];
    assert_eq!(f.max_nesting, 5);
}

#[test]
fn only_the_outermost_member_records_a_chain() {
    let src = b"import * as ns from './api';\nfunction f(o) {\n  return o.a.b.c.d + ns.x.y.z;\n}\n";
    let functions = parse_source(src, 0, "m", &JsProfile)
        .unwrap()
        .units
        .functions;
    let hits: Vec<(usize, usize, Option<&str>)> = functions[0]
        .chain_hits
        .iter()
        .map(|h| (h.line, h.depth, h.root.as_deref()))
        .collect();
    assert_eq!(
        hits,
        vec![(3, 4, Some("o")), (3, 2, Some("ns"))],
        "one hit per chain; the namespace hop is not navigation"
    );
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

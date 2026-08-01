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
    assert_eq!(functions[0].magic_numbers, 0);
    assert_eq!(functions[1].magic_numbers, 1);
    assert_eq!(functions[0].branch_count, 1);
}

/// Parent function metrics exclude nested arrow and declaration bodies, while
/// each nested unit tracks its own scope.
#[test]
fn fused_collector_nested_arrows_and_declarations() {
    let src = b"function top(x) {\n  const f = () => x + 1;\n  function helper(y) { return y * 2; }\n  return f(helper(x));\n}\n";
    let functions = parse_source(src, 0, "m", &JsProfile)
        .unwrap()
        .units
        .functions;

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "top");
    // arrow appears first in body (variable_declarator → arrow_function),
    // then the named function_declaration helper.
    assert_eq!(functions[1].name, "");
    assert_eq!(functions[2].name, "helper");
    assert!(functions[2].is_nested);
    assert_eq!(functions[2].parent, "top");

    // top's return count = 1 (the `return f(helper(x))`) — not the returns
    // inside helper or the arrow.
    assert_eq!(functions[0].return_count, 1);
    assert_eq!(functions[2].return_count, 1); // helper's own return
    assert_eq!(functions[0].branch_count, 0);
}

/// Nested loop inside a conditional: cognitive complexity, nesting depth, and
/// loop-call facts are attributed to the correct nesting context.
#[test]
fn fused_collector_nested_loop_and_conditional() {
    let src =
        b"function scan(items) {\n  if (items.length) {\n    for (const x of items) {\n      log(x);\n    }\n  }\n}\n";
    let functions = parse_source(src, 0, "m", &JsProfile)
        .unwrap()
        .units
        .functions;

    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    // nesting: `if` + `for` inside body → max_nesting = 2
    assert_eq!(f.max_nesting, 2);
    // cognitive: if (1+0) + for (1+1) = 3
    assert_eq!(f.cognitive, 3);
    // branches: if_statement + for_in_statement = 2
    assert_eq!(f.branch_count, 2);
    // one call `log(x)` inside a loop
    assert_eq!(f.performance.loop_calls.len(), 1);
    assert_eq!(f.performance.loops.len(), 1);
}

/// Class method: is_method is true, `this.attr` access populates self_attrs,
/// and methods appear in source order.
#[test]
fn fused_collector_class_method() {
    let src = b"class Store {\n  save(data) { this.buffer = data; return this.ok; }\n  load() { return this.buffer; }\n}\n";
    let units = parse_source(src, 0, "m", &JsProfile).unwrap().units;

    assert_eq!(units.classes.len(), 1);
    assert_eq!(units.classes[0].name, "Store");
    assert_eq!(units.classes[0].methods, vec!["save", "load"]);

    assert_eq!(units.functions.len(), 2);
    let save = &units.functions[0];
    assert_eq!(save.name, "save");
    assert!(save.is_method);
    assert_eq!(save.return_count, 1);
    assert!(save.self_attrs.contains("buffer"));
    assert!(save.self_attrs.contains("ok"));

    let load = &units.functions[1];
    assert_eq!(load.name, "load");
    assert!(load.is_method);
    assert_eq!(load.return_count, 1);
    assert!(load.self_attrs.contains("buffer"));
    // receiver_access credits `this` as "self"
    assert!(save.receiver_access.contains_key("self"));
    assert!(load.receiver_access.contains_key("self"));
}

/// Sequential parse of a fixture produces identical debug representation before
/// and after the traversal fusion (smell detectors run from the same IR).
#[test]
fn fused_collector_json_identity() {
    let src = b"function compute(n) {\n  let s = 0;\n  for (let i = 0; i < n; i++) {\n    if (i % 2) { s += i; } else { s += 1; }\n  }\n  return s;\n}\n";
    let functions = parse_source(src, 0, "m", &JsProfile)
        .unwrap()
        .units
        .functions;

    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    assert_eq!(f.name, "compute");
    assert_eq!(f.max_nesting, 2);
    assert_eq!(f.return_count, 1);
    assert_eq!(f.branch_count, 2);
    assert_eq!(f.cognitive, 3);
    assert_eq!(f.magic_numbers, 0);
    assert_eq!(f.collapsible_nested_ifs, 0);
    assert_eq!(f.comment_lines, 0);
    assert_eq!(f.max_chain_depth, 0);
    assert!(!f.is_method);
    assert!(!f.is_nested);
    assert_eq!(f.parent, "");
    assert_eq!(f.param_names, vec!["n"]);
    assert_eq!(f.max_tuple_return, 0);
    assert!(f.local_reassigns.is_empty());
    assert!(f.receiver_access.is_empty());
    assert!(f.self_attrs.is_empty());
    assert!(f.own_method_calls.is_empty());
    assert!(f.str_keys.is_empty());
    assert!(f.schema_calls.is_empty());
    assert!(f.validated_names.is_empty());
    assert!(f.returned_constructors.is_empty());
    assert!(f.mutated_names.is_empty());
    assert!(f.attr_mutated_names.is_empty());
    assert_eq!(f.performance.loops.len(), 1);
    assert_eq!(f.performance.loops[0].line, 3);
    assert!(f.performance.nested_loops.is_empty());
    assert!(f.performance.calls.is_empty());
    assert!(f.performance.loop_calls.is_empty());
    assert!(f.performance.iteration_calls.is_empty());
    assert!(f.performance.sorts_in_loops.is_empty());
    assert_eq!(f.review_risks.broad_handlers, 0);
    assert_eq!(f.review_risks.empty_fallbacks, 0);
    assert_eq!(f.review_risks.repeated_guards, 0);
    assert_eq!(f.literal_membership_tests, 0);
    assert!(f.short_string_fallback_lines.is_empty());
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

//! Rust profile behavior tests (structural pillars: tokens, imports,
//! resolution, cycles, duplication, dead-code crediting).

use super::RustProfile;
use crate::spine::parser::{parse_file, parse_source, StructuralToken};
use std::fs;
use std::path::Path;

fn tokens(src: &[u8]) -> Vec<StructuralToken> {
    parse_source(src, 0, "m", &RustProfile)
        .unwrap()
        .syntax
        .tokens
}

/// Two functions with the same control-flow shape but different local names
/// produce identical structural-token vectors (rename invariance).
#[test]
fn rename_invariance_yields_identical_tokens() {
    let a = b"fn compute(items: &[i64]) -> i64 {\n    let mut total = 0;\n    for item in items {\n        if *item > 10 { total += item; }\n    }\n    total\n}\n";
    let b = b"fn process(values: &[i64]) -> i64 {\n    let mut acc = 0;\n    for value in values {\n        if *value > 10 { acc += value; }\n    }\n    acc\n}\n";
    let ta = tokens(a);
    assert_eq!(ta, tokens(b), "renamed-but-identical fns must match");
    assert!(ta.contains(&StructuralToken::FunctionDef));
    assert!(ta.contains(&StructuralToken::ForStatement));
    assert!(ta.contains(&StructuralToken::IfStatement));
}

/// `use` trees flatten: groups, aliases, globs, `self`, and `mod` decls.
#[test]
fn use_tree_flattening() {
    let src = b"mod helpers;\nuse crate::noze::{AnalysisReport, smells};\nuse std::collections::HashMap as Map;\nuse crate::config::*;\nuse serde_json;\n";
    let imports = parse_source(src, 0, "m", &RustProfile)
        .unwrap()
        .symbols
        .imports;
    let targets: Vec<&str> = imports.iter().map(|i| i.target_module.as_str()).collect();
    assert!(targets.contains(&"self::helpers"), "mod decl is an edge");
    assert!(targets.contains(&"crate::noze"));
    assert!(targets.contains(&"std::collections"));
    assert!(targets.contains(&"crate::config"));
    assert!(targets.contains(&"serde_json"));

    let grouped: Vec<_> = imports
        .iter()
        .filter(|i| i.target_module == "crate::noze")
        .collect();
    let symbols: Vec<&str> = grouped
        .iter()
        .flat_map(|i| i.imported_symbols.iter().map(String::as_str))
        .collect();
    assert_eq!(symbols, vec!["AnalysisReport", "smells"]);

    let aliased = imports
        .iter()
        .find(|i| i.target_module == "std::collections")
        .unwrap();
    assert_eq!(aliased.imported_symbols, vec!["HashMap"]);
    assert_eq!(aliased.bindings, vec!["Map"], "alias-aware binding");

    let glob = imports
        .iter()
        .find(|i| i.target_module == "crate::config")
        .unwrap();
    assert_eq!(glob.imported_symbols, vec!["*"]);
}

/// `crate::`/`self::`/`super::` and the package's own name resolve to in-repo
/// module keys; `std`/third-party stay external.
#[test]
fn path_resolution() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("src/noze")).unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]\nname = \"my-crate\"\n").unwrap();
    fs::write(dir.join("src/lib.rs"), "").unwrap();

    let resolve = |target: &str, importer: &Path| {
        let ctx = crate::spine::parser::ImportContext {
            source_module: "x".into(),
            target_module: target.into(),
            imported_symbols: vec![],
            bindings: vec![],
            line: 1,
            column: 1,
            is_inline: false,
            is_module_decl: false,
            enclosing_scope: None,
        };
        super::resolve::resolve_target(&ctx, importer, &dir)
    };

    let cycles = dir.join("src/noze/cycles.rs");
    assert_eq!(resolve("crate::config", &cycles), "src/config");
    assert_eq!(resolve("self::tests", &cycles), "src/noze/cycles/tests");
    assert_eq!(resolve("super::boundaries", &cycles), "src/noze/boundaries");
    assert_eq!(resolve("super::super::config", &cycles), "src/config");
    // The package's own name (normalized) acts like `crate` — also from a
    // sibling crate dir like tests/.
    assert_eq!(
        resolve("my_crate::config", &dir.join("tests/it.rs")),
        "src/config"
    );
    assert_eq!(resolve("std::collections", &cycles), "std::collections");
}

/// Mutual `use` between two modules forms a detected cycle, and `mod` decls
/// from lib.rs resolve to the module files.
#[test]
fn mutual_use_is_a_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]\nname = \"c\"\n").unwrap();
    fs::write(dir.join("src/lib.rs"), "pub mod a;\npub mod b;\n").unwrap();
    fs::write(
        dir.join("src/a.rs"),
        "use crate::b::two;\npub fn one() -> u32 { two() }\n",
    )
    .unwrap();
    fs::write(
        dir.join("src/b.rs"),
        "use crate::a::one;\npub fn two() -> u32 { one() }\n",
    )
    .unwrap();

    let files: Vec<_> = ["src/lib.rs", "src/a.rs", "src/b.rs"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    assert!(cg.name_to_index.contains_key("src"), "lib.rs keys as src");
    let lib = cg.name_to_index["src"];
    let a = cg.name_to_index["src/a"];
    assert!(
        cg.graph.find_edge(lib, a).is_some(),
        "mod decl creates lib -> a edge"
    );

    let cycles = crate::noze::cycles::detect(&cg, &[]);
    assert_eq!(cycles.len(), 1, "src/a <-> src/b is a circular use");
}

/// Parent `mod child;` + child `use super::…` is the idiomatic Rust module
/// hierarchy — containment, not coupling — and must NOT be reported as a
/// circular import (caught by scanning sensez itself).
#[test]
fn module_hierarchy_is_not_a_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("src/p")).unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]\nname = \"h\"\n").unwrap();
    fs::write(dir.join("src/lib.rs"), "pub mod p;\n").unwrap();
    fs::write(
        dir.join("src/p/mod.rs"),
        "pub mod child;\npub fn thing() {}\n",
    )
    .unwrap();
    fs::write(
        dir.join("src/p/child.rs"),
        "use super::thing;\npub fn go() {\n    thing()\n}\n",
    )
    .unwrap();

    let files: Vec<_> = ["src/lib.rs", "src/p/mod.rs", "src/p/child.rs"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    let p = cg.name_to_index["src/p"];
    let child = cg.name_to_index["src/p/child"];
    assert!(
        cg.graph.find_edge(p, child).is_some() && cg.graph.find_edge(child, p).is_some(),
        "both hierarchy and use edges exist (dead-code still credits them)"
    );

    let cycles = crate::noze::cycles::detect(&cg, &[]);
    assert!(
        cycles.is_empty(),
        "mod decl + use super:: is containment, not a cycle: {cycles:?}"
    );
}

/// The façade pattern — parent re-exports its child's API (`pub use
/// child::go`) while the child uses the parent's types (`use super::T`) — is
/// idiomatic Rust, not a circular import (caught by scanning sensez itself).
#[test]
fn facade_reexport_is_not_a_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("src/p")).unwrap();
    fs::write(dir.join("Cargo.toml"), "[package]\nname = \"f\"\n").unwrap();
    fs::write(dir.join("src/lib.rs"), "pub mod p;\n").unwrap();
    fs::write(
        dir.join("src/p/mod.rs"),
        "mod child;\npub use child::go;\npub struct T;\n",
    )
    .unwrap();
    fs::write(
        dir.join("src/p/child.rs"),
        "use super::T;\npub fn go() -> T {\n    T\n}\n",
    )
    .unwrap();

    let files: Vec<_> = ["src/lib.rs", "src/p/mod.rs", "src/p/child.rs"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    let cycles = crate::noze::cycles::detect(&cg, &[]);
    assert!(
        cycles.is_empty(),
        "façade re-export + use super:: is containment, not a cycle: {cycles:?}"
    );
}

/// Two byte-identical Rust functions in different files are a clone; a Python
/// function with the same shape never joins them (per-language partition).
#[test]
fn identical_functions_are_a_clone_within_rust_only() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    let body = "pub fn handle(req: u32, code: u32) -> u32 {\n    let data = req + code;\n    if data > 0 {\n        send(data);\n    } else {\n        fail(code);\n    }\n    data + code\n}\n";
    fs::write(dir.join("one.rs"), body).unwrap();
    fs::write(dir.join("two.rs"), body).unwrap();

    let files: Vec<_> = ["one.rs", "two.rs"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cfg = crate::config::model::Duplication {
        exclude: vec![],
        threshold: 10,
        max_gap: 0,
        near_miss: false,
        class_property_overlap_min: 4,
    };
    let dup = crate::noze::duplication::detect(&files, &cfg);
    assert!(!dup.is_empty(), "identical Rust functions must be a clone");
}

/// Only `pub` items are declared (rustc owns private dead code), and a module
/// used solely via `module::func()` paths is credited, not flagged.
#[test]
fn pub_only_declarations_and_path_access_crediting() {
    let src = b"pub fn used() {}\npub fn unused() {}\nfn private() {}\npub struct Conf;\n";
    let walked = parse_source(src, 0, "m", &RustProfile).unwrap();
    assert_eq!(walked.symbols.declared, vec!["used", "unused", "Conf"]);
    assert_eq!(
        walked.symbols.declared_kinds["Conf"],
        crate::spine::parser::SymbolKind::Class
    );
    assert!(
        !walked.symbols.declared.contains(&"private".to_string()),
        "private items are rustc's job"
    );

    let caller = parse_source(
        b"use crate::m;\npub fn go() { m::used(); }\n",
        1,
        "caller",
        &RustProfile,
    )
    .unwrap();
    assert!(
        caller.usage.attribute_accesses["m"].contains("used"),
        "path call credited as attribute access"
    );
}

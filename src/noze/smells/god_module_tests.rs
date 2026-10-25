//! `god_module` needs a sizeable, balanced hub of internal runtime edges.

use super::graphy;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::parser::parse_file;
use std::fs;

/// God-module file names for a flat TS project given as `(name, body)` pairs.
fn god_modules(files: &[(String, String)], cfg: Smells) -> Vec<String> {
    god_module_findings(files, cfg)
        .into_iter()
        .map(|f| f.symbol)
        .collect()
}

fn god_module_findings(files: &[(String, String)], cfg: Smells) -> Vec<SmellFinding> {
    let tmp = tempfile::tempdir().unwrap();
    let parsed: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(i, (name, body))| {
            let path = tmp.path().join(format!("{name}.ts"));
            fs::write(&path, body).unwrap();
            parse_file(&path, i as u32).unwrap()
        })
        .collect();
    let graph = crate::spine::graph::build(&parsed, &[]);
    graphy::detect(&graph, &parsed, &cfg.into(), &|_| true)
        .into_iter()
        .filter(|f| f.kind == SmellKind::GodModule)
        .collect()
}

fn no_shotgun() -> Smells {
    Smells {
        shotgun_blast_threshold: 1000,
        ..Smells::default()
    }
}

fn file(name: &str, body: impl Into<String>) -> (String, String) {
    (name.to_string(), body.into())
}

fn importers(prefix: &str, count: usize, target: &str) -> Vec<(String, String)> {
    (0..count)
        .map(|i| {
            file(
                &format!("{prefix}{i}"),
                format!("import {{ x }} from \"./{target}\";\nexport const y{i} = x;\n"),
            )
        })
        .collect()
}

#[test]
fn small_widely_imported_leaf_is_not_a_god_module() {
    let mut files = vec![
        file("theme", "export type Theme = { dark: boolean };\n"),
        file(
            "Button",
            "import React from \"react\";\nimport clsx from \"clsx\";\nimport { Slot } from \"@radix-ui/react-slot\";\nimport { cva } from \"class-variance-authority\";\nimport type { Theme } from \"./theme\";\nexport const x = [React, clsx, Slot, cva];\nexport function paint(t: Theme) {\n  return t.dark;\n}\n",
        ),
    ];
    files.extend(importers("page", 22, "Button"));
    files.extend(importers("button_test", 5, "Button"));
    assert!(god_modules(&files, no_shotgun()).is_empty());
}

#[test]
fn thin_page_composing_many_siblings_is_not_a_god_module() {
    let mut page = String::new();
    let mut files = Vec::new();
    for i in 0..20 {
        page.push_str(&format!("import {{ C{i} }} from \"./c{i}\";\n"));
        files.push(file(
            &format!("c{i}"),
            format!("export const C{i} = {i};\n"),
        ));
    }
    page.push_str("export const x = 1;\n");
    files.push(file("Page", page));
    files.extend(importers("route", 5, "Page"));
    files.push(file("fixtures", "export const x = 1;\n"));
    files.extend(importers("spec", 30, "fixtures"));
    assert!(god_modules(&files, no_shotgun()).is_empty());

    let no_size_gate = Smells {
        god_module_min_lines: 0,
        god_module_min_units: 0,
        ..no_shotgun()
    };
    assert_eq!(god_modules(&files, no_size_gate), vec!["Page".to_string()]);
}

/// A 400-line `service` with 12 internal runtime imports, plus its dependencies.
fn hub() -> Vec<(String, String)> {
    let mut service = String::new();
    let mut files = Vec::new();
    for i in 0..12 {
        service.push_str(&format!("import {{ d{i} }} from \"./dep{i}\";\n"));
        files.push(file(
            &format!("dep{i}"),
            format!("export const d{i} = {i};\n"),
        ));
    }
    for i in 0..400 {
        service.push_str(&format!("export const v{i} = {i};\n"));
    }
    service.push_str("export const x = d0;\nexport function total() {\n  return x;\n}\n");
    files.push(file("service", service));
    files
}

/// A 400-line module of imported tables composed into one object, with no functions.
fn aggregator() -> Vec<(String, String)> {
    let mut schema = String::new();
    let mut files = Vec::new();
    for i in 0..12 {
        schema.push_str(&format!("import {{ t{i} }} from \"./table{i}\";\n"));
        files.push(file(
            &format!("table{i}"),
            format!("export const t{i} = {i};\n"),
        ));
    }
    schema.push_str("export const x = {\n");
    for i in 0..400 {
        schema.push_str(&format!("  k{i}: t{},\n", i % 12));
    }
    schema.push_str("};\n");
    files.push(file("schema", schema));
    files
}

#[test]
fn composition_root_without_functions_is_not_a_god_module() {
    let mut files = aggregator();
    files.extend(importers("caller", 15, "schema"));
    assert!(god_modules(&files, no_shotgun()).is_empty());
}

#[test]
fn god_module_finding_is_anchored_with_advice() {
    let mut files = hub();
    files.extend(importers("caller", 15, "service"));
    let finding = god_module_findings(&files, no_shotgun())
        .pop()
        .expect("hub is a god module");
    assert_eq!(finding.line, 1);
    assert!(
        finding.message.contains("split it by responsibility"),
        "{}",
        finding.message
    );
}

#[test]
fn large_balanced_hub_is_a_god_module() {
    let mut files = hub();
    files.extend(importers("caller", 15, "service"));
    assert_eq!(
        god_modules(&files, no_shotgun()),
        vec!["service".to_string()]
    );
}

#[test]
fn type_only_importers_are_not_fan_in() {
    let mut files = hub();
    files.extend((0..15).map(|i| {
        file(
            &format!("caller{i}"),
            format!("import type {{ x }} from \"./service\";\nexport type Y{i} = typeof x;\n"),
        )
    }));
    assert!(god_modules(&files, no_shotgun()).is_empty());
}

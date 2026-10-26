//! `god_module` needs a sizeable, balanced hub of internal runtime edges.

use super::graphy;
use crate::config::smells::Smells;
use crate::report::SmellKind;
use crate::spine::parser::parse_file;
use std::fs;

/// God-module file names for a flat TS project given as `(name, body)` pairs.
fn god_modules(files: &[(String, String)], cfg: Smells) -> Vec<String> {
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
        .map(|f| f.symbol)
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
        ..no_shotgun()
    };
    assert_eq!(god_modules(&files, no_size_gate), vec!["Page".to_string()]);
}

#[test]
fn large_balanced_hub_is_a_god_module() {
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
    service.push_str("export const x = d0;\n");
    files.push(file("service", service));
    files.extend(importers("caller", 15, "service"));
    assert_eq!(
        god_modules(&files, no_shotgun()),
        vec!["service".to_string()]
    );
}

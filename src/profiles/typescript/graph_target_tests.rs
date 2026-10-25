use crate::spine::parser::parse_file;
use crate::test_support::write;
use petgraph::visit::EdgeRef;
use std::path::Path;

/// File paths of the internal modules `importer` has an edge to.
fn internal_targets(root: &Path, files: &[(&str, &str)], importer: &str) -> Vec<String> {
    let parsed: Vec<_> = files
        .iter()
        .enumerate()
        .filter(|(_, (path, _))| !path.ends_with(".json"))
        .map(|(i, (path, _))| parse_file(&root.join(path), i as u32).unwrap())
        .collect();
    let graph = crate::spine::graph::build(&parsed, &[]);
    let from = graph
        .graph
        .node_indices()
        .find(|index| graph.graph[*index].file_path == root.join(importer))
        .unwrap();
    graph
        .graph
        .edges(from)
        .map(|edge| &graph.graph[edge.target()])
        .filter(|node| !node.is_external)
        .map(|node| {
            node.file_path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn targets(files: &[(&str, &str)], importer: &str) -> Vec<String> {
    let tmp = tempfile::tempdir().unwrap();
    for (path, source) in files {
        write(tmp.path(), path, source);
    }
    internal_targets(tmp.path(), files, importer)
}

#[test]
fn relative_import_crosses_a_nested_package_marker() {
    let files = [
        ("package.json", r#"{"name":"app"}"#),
        ("src/legacy/package.json", r#"{"type":"commonjs"}"#),
        ("src/legacy/thing.js", "module.exports = 1;\n"),
        (
            "src/app/main.ts",
            "import thing from '../legacy/thing';\nexport const x = thing;\n",
        ),
    ];

    assert_eq!(targets(&files, "src/app/main.ts"), ["src/legacy/thing.js"]);
}

#[test]
fn root_tsconfig_alias_applies_inside_a_nested_package() {
    let files = [
        ("package.json", r#"{"name":"repo"}"#),
        (
            "tsconfig.json",
            r#"{ "compilerOptions": { "paths": { "@web/*": ["packages/web/src/*"] } } }"#,
        ),
        ("packages/web/package.json", r#"{"name":"web"}"#),
        ("packages/web/src/util.ts", "export const one = 1;\n"),
        (
            "packages/web/src/sum.ts",
            "import { one } from '@web/util';\nexport const two = one + one;\n",
        ),
    ];

    assert_eq!(
        targets(&files, "packages/web/src/sum.ts"),
        ["packages/web/src/util.ts"]
    );
}

#[test]
fn alias_target_outside_the_package_root_is_internal() {
    let files = [
        ("packages/core/package.json", r#"{"name":"core"}"#),
        ("packages/core/src/index.ts", "export const core = 1;\n"),
        ("packages/web/package.json", r#"{"name":"web"}"#),
        (
            "packages/web/tsconfig.json",
            r#"{ "compilerOptions": { "paths": { "@acme/core": ["../core/src/index.ts"] } } }"#,
        ),
        (
            "packages/web/src/sum.ts",
            "import { core } from '@acme/core';\nexport const two = core;\n",
        ),
    ];

    assert_eq!(
        targets(&files, "packages/web/src/sum.ts"),
        ["packages/core/src/index.ts"]
    );
}

#[test]
fn alias_falls_through_to_a_later_target() {
    let files = [
        ("package.json", r#"{"name":"app"}"#),
        (
            "tsconfig.json",
            r#"{ "compilerOptions": { "paths": { "@/*": ["src/*", "generated/*"] } } }"#,
        ),
        ("generated/api.ts", "export const api = 1;\n"),
        (
            "src/main.ts",
            "import { api } from '@/api';\nexport const x = api;\n",
        ),
    ];

    assert_eq!(targets(&files, "src/main.ts"), ["generated/api.ts"]);
}

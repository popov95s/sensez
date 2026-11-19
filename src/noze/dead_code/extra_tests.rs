use super::*;
use crate::spine::parser::parse_file;
use std::fs;

#[test]
fn detects_unused_import_and_method() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("m.py"),
        "import os\nimport sys\n\nsys.exit(0)\n\nclass C:\n    def used(self):\n        return self.unused()\n    def unused(self):\n        return 1\n    def orphan(self):\n        return 2\n",
    )
    .unwrap();
    let files = vec![parse_file(&dir.join("m.py"), 0).unwrap()];
    let cg = crate::spine::graph::build(&files, &[]);
    let modmap = module_map(&cg);

    let imps: Vec<_> = unused_imports(&files, &modmap)
        .iter()
        .map(|f| f.symbol.clone())
        .collect();
    assert!(
        imps.contains(&"os".to_string()),
        "os is imported but unused"
    );
    assert!(!imps.contains(&"sys".to_string()), "sys is used");

    let member_files = MemberFiles::same(files.iter());
    let meths: Vec<_> = unused_methods(&member_files, &modmap, |_language, _name| false)
        .iter()
        .map(|f| f.symbol.clone())
        .collect();
    assert!(
        meths.contains(&"orphan".to_string()),
        "orphan() is never called"
    );
    assert!(
        !meths.contains(&"unused".to_string()),
        "unused() is called via self"
    );
}

#[test]
fn ignores_type_checking_only_imports() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("m.py"),
        "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from app.models import User\n\nimport os\n",
    )
    .unwrap();
    let files = vec![parse_file(&dir.join("m.py"), 0).unwrap()];
    let cg = crate::spine::graph::build(&files, &[]);
    let modmap = module_map(&cg);

    let imps: Vec<_> = unused_imports(&files, &modmap)
        .iter()
        .map(|f| f.symbol.clone())
        .collect();
    assert!(
        !imps.contains(&"User".to_string()),
        "type-checker-only imports are intentional"
    );
    assert!(imps.contains(&"os".to_string()), "ordinary unused import");
}

#[cfg(feature = "lang-typescript")]
#[test]
fn ignores_type_only_bindings_but_reports_runtime_bindings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("consumer.ts"),
        "import { type MassiveUserClass, connect as runtimeConnect } from './models';\n",
    )
    .unwrap();
    fs::write(
        dir.join("models.ts"),
        "export class MassiveUserClass {}\nexport function connect(): void {}\n",
    )
    .unwrap();
    let files = vec![
        parse_file(&dir.join("consumer.ts"), 0).unwrap(),
        parse_file(&dir.join("models.ts"), 1).unwrap(),
    ];
    let cg = crate::spine::graph::build(&files, &[]);
    let modmap = module_map(&cg);

    let imps: Vec<_> = unused_imports(&files, &modmap)
        .iter()
        .map(|f| f.symbol.clone())
        .collect();
    assert!(!imps.contains(&"MassiveUserClass".to_string()));
    assert!(imps.contains(&"runtimeConnect".to_string()));
}

#[cfg(feature = "lang-typescript")]
#[test]
fn recognizes_runtime_imports_referenced_only_by_types() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("consumer.ts"),
        "import { Shape, unused } from './models';\ntype Item = Shape;\n",
    )
    .unwrap();
    fs::write(
        dir.join("models.ts"),
        "export interface Shape { readonly id: string; }\nexport const unused = 1;\n",
    )
    .unwrap();
    let files = vec![
        parse_file(&dir.join("consumer.ts"), 0).unwrap(),
        parse_file(&dir.join("models.ts"), 1).unwrap(),
    ];
    let graph = crate::spine::graph::build(&files, &[]);
    let modmap = module_map(&graph);

    let findings = unused_imports(&files, &modmap);
    let symbols: Vec<_> = findings
        .iter()
        .map(|finding| finding.symbol.as_str())
        .collect();
    assert!(!symbols.contains(&"Shape"));
    assert!(symbols.contains(&"unused"));
}

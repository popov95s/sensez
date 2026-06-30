use super::*;
use crate::spine::parser::parse_file;
use std::fs;
use std::path::Path;

fn cfg() -> DeadCode {
    DeadCode {
        entrypoints: vec![],
        entrypoint_names: vec![],
        entrypoint_bases: vec![],
        entry_points: vec![],
        entry_modules: vec![],
        unused_imports: false,
        unused_methods: false,
        unused_properties: false,
        unused_variables: false,
    }
}

fn dead_symbols(dir: &Path, names: &[&str]) -> Vec<(String, String)> {
    let files: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    detect(&cg, &files, &cfg())
        .iter()
        .map(|finding| (finding.module.clone(), finding.symbol.clone()))
        .collect()
}

#[test]
fn test_only_imports_do_not_keep_symbols_alive() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::write(
        dir.join("lib.py"),
        "def only_tested():\n    return 1\n\n\ndef app_used():\n    return 2\n",
    )
    .unwrap();
    fs::write(
        dir.join("tests/test_lib.py"),
        "from lib import only_tested\n\n\ndef test_only_tested():\n    assert only_tested() == 1\n",
    )
    .unwrap();
    fs::write(
        dir.join("app.py"),
        "from lib import app_used\n\n\ndef run():\n    return app_used()\n",
    )
    .unwrap();

    let dead = dead_symbols(&dir, &["lib.py", "tests/test_lib.py", "app.py"]);

    assert!(
        dead.contains(&("lib".into(), "only_tested".into())),
        "symbols imported only by tests should remain dead candidates; got {dead:?}"
    );
    assert!(
        !dead.contains(&("lib".into(), "app_used".into())),
        "application imports should still keep symbols alive; got {dead:?}"
    );
}

#[test]
fn type_checking_imports_keep_symbols_alive() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("models.py"),
        "class TypeOnlyModel:\n    pass\n\n\nclass RuntimeDead:\n    pass\n",
    )
    .unwrap();
    fs::write(
        dir.join("consumer.py"),
        "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from models import TypeOnlyModel\n",
    )
    .unwrap();

    let dead = dead_symbols(&dir, &["models.py", "consumer.py"]);

    assert!(
        !dead.contains(&("models".into(), "TypeOnlyModel".into())),
        "TYPE_CHECKING imports are real type usage; got {dead:?}"
    );
    assert!(
        dead.contains(&("models".into(), "RuntimeDead".into())),
        "other symbols in the same imported module should still be candidates; got {dead:?}"
    );
}

#[test]
fn test_sources_do_not_emit_dead_code_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::write(
        dir.join("prod.py"),
        "def production_dead():\n    return 1\n",
    )
    .unwrap();
    fs::write(
        dir.join("tests/test_helpers.py"),
        "import os\n\nclass Helper:\n    stale: str\n\n    def orphan(self):\n        return 1\n\n\ndef test_helper():\n    return Helper()\n",
    )
    .unwrap();

    let files = vec![
        parse_file(&dir.join("prod.py"), 0).unwrap(),
        parse_file(&dir.join("tests/test_helpers.py"), 1).unwrap(),
    ];
    let cg = crate::spine::graph::build(&files, &[]);
    let mut config = cfg();
    config.unused_imports = true;
    config.unused_methods = true;
    config.unused_properties = true;

    let dead: Vec<_> = detect(&cg, &files, &config)
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect();

    assert!(dead.contains(&"production_dead".to_string()));
    assert!(
        !dead.iter().any(|symbol| is_test_only_symbol(symbol)),
        "test/spec files should not emit dead-code findings; got {dead:?}"
    );
}

#[test]
fn entry_point_files_do_not_emit_dead_code_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("public_api.py"),
        "import os\n\nPUBLIC_CONST = 1\n\nclass PublicApi:\n    stale: str\n\n    def extension_hook(self):\n        return 1\n\n\ndef extension_fn():\n    return PublicApi()\n",
    )
    .unwrap();
    fs::write(
        dir.join("internal.py"),
        "def internal_dead():\n    return 1\n",
    )
    .unwrap();

    let files = vec![
        parse_file(&dir.join("public_api.py"), 0).unwrap(),
        parse_file(&dir.join("internal.py"), 1).unwrap(),
    ];
    let cg = crate::spine::graph::build(&files, &[]);
    let mut config = cfg();
    config.entry_points = vec!["**/public_api.py".to_string()];
    config.unused_imports = true;
    config.unused_methods = true;
    config.unused_properties = true;
    config.unused_variables = true;

    let dead: Vec<_> = detect(&cg, &files, &config)
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect();

    assert!(dead.contains(&"internal_dead".to_string()));
    assert!(
        !dead.iter().any(|symbol| is_public_api_symbol(symbol)),
        "entry-point files should not emit dead-code findings; got {dead:?}"
    );
}

fn is_test_only_symbol(symbol: &str) -> bool {
    symbol == "os"
        || symbol == "Helper"
        || symbol == "Helper.stale"
        || symbol == "orphan"
        || symbol == "test_helper"
}

fn is_public_api_symbol(symbol: &str) -> bool {
    symbol == "os"
        || symbol == "PUBLIC_CONST"
        || symbol == "PublicApi"
        || symbol == "PublicApi.stale"
        || symbol == "extension_hook"
        || symbol == "extension_fn"
}

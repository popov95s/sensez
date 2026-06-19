use super::*;
use crate::spine::parser::parse_file;
use std::fs;

fn cfg() -> DeadCode {
    DeadCode {
        entrypoints: vec![],
        entrypoint_names: vec![],
        entrypoint_bases: vec![],
        entry_points: vec![],
        entry_modules: vec![],
        unused_imports: false,
        unused_methods: false,
        unused_variables: false,
    }
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

    let names = ["lib.py", "tests/test_lib.py", "app.py"];
    let files: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    let dead: Vec<_> = detect(&cg, &files, &cfg())
        .iter()
        .map(|finding| (finding.module.clone(), finding.symbol.clone()))
        .collect();

    assert!(
        dead.contains(&("lib".into(), "only_tested".into())),
        "symbols imported only by tests should remain dead candidates; got {dead:?}"
    );
    assert!(
        !dead.contains(&("lib".into(), "app_used".into())),
        "application imports should still keep symbols alive; got {dead:?}"
    );
}

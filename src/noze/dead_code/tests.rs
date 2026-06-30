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
        unused_properties: false,
        unused_variables: false,
    }
}

#[test]
fn configured_class_bases_mark_dynamic_entrypoints() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("apps.py"),
        "from framework import AppConfig\n\n\
         class SimpleAdminConfig(AppConfig):\n    pass\n\n\
         class AdminConfig(SimpleAdminConfig):\n    pass\n\n\
         class Plain:\n    pass\n",
    )
    .unwrap();

    let files = vec![parse_file(&dir.join("apps.py"), 0).unwrap()];
    let cg = crate::spine::graph::build(&files, &[]);
    let mut cfg = cfg();
    cfg.entrypoint_bases = vec!["AppConfig".to_string()];

    let dead: Vec<_> = detect(&cg, &files, &cfg)
        .iter()
        .map(|f| f.symbol.clone())
        .collect();

    assert!(!dead.contains(&"SimpleAdminConfig".to_string()));
    assert!(!dead.contains(&"AdminConfig".to_string()));
    assert!(dead.contains(&"Plain".to_string()));
}

#[test]
fn python_profile_class_bases_mark_dynamic_entrypoints_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("apps.py"),
        "from framework import AppConfig\n\n\
         class SimpleAdminConfig(AppConfig):\n    pass\n\n\
         class AdminConfig(SimpleAdminConfig):\n    pass\n\n\
         class Plain:\n    pass\n",
    )
    .unwrap();

    let files = vec![parse_file(&dir.join("apps.py"), 0).unwrap()];
    let cg = crate::spine::graph::build(&files, &[]);
    let dead: Vec<_> = detect(
        &cg,
        &files,
        &crate::config::model::Config::default().dead_code,
    )
    .iter()
    .map(|f| f.symbol.clone())
    .collect();

    assert!(!dead.contains(&"SimpleAdminConfig".to_string()));
    assert!(!dead.contains(&"AdminConfig".to_string()));
    assert!(dead.contains(&"Plain".to_string()));
}

#[test]
fn flags_unused_functions_only_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("lib.py"),
        "DEAD_CONST = 1\n\ndef used_fn():\n    pass\n\ndef dead_fn():\n    pass\n",
    )
    .unwrap();
    fs::write(dir.join("app.py"), "from lib import used_fn\n\nused_fn()\n").unwrap();

    let files: Vec<_> = ["lib.py", "app.py"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);

    let dead: Vec<_> = detect(&cg, &files, &cfg())
        .iter()
        .map(|f| f.symbol.clone())
        .collect();
    assert!(dead.contains(&"dead_fn".to_string()));
    assert!(!dead.contains(&"used_fn".to_string()));
    // module-level variable is OFF by default
    assert!(!dead.contains(&"DEAD_CONST".to_string()));

    let mut on = cfg();
    on.unused_variables = true;
    let dead2: Vec<_> = detect(&cg, &files, &on)
        .iter()
        .map(|f| f.symbol.clone())
        .collect();
    assert!(dead2.contains(&"DEAD_CONST".to_string()));
}

#[test]
fn unused_properties_are_opt_in_dead_code() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("model.py"),
        "class User:\n    name: str\n    stale: str\n\n    def label(self):\n        return self.name\n",
    )
    .unwrap();

    let files = vec![parse_file(&dir.join("model.py"), 0).unwrap()];
    let cg = crate::spine::graph::build(&files, &[]);

    let off: Vec<_> = detect(&cg, &files, &cfg())
        .iter()
        .map(|f| f.symbol.clone())
        .collect();
    assert!(!off.contains(&"User.stale".to_string()));

    let mut on = cfg();
    on.unused_properties = true;
    let dead: Vec<_> = detect(&cg, &files, &on)
        .iter()
        .map(|f| (f.symbol.clone(), f.kind))
        .collect();
    assert!(dead.contains(&("User.stale".to_string(), SymbolKind::Property)));
}

/// Python profile defaults treat alembic migrations and test files as entry
/// points: their symbols aren't flagged dead (external runners call them), but
/// real dead code elsewhere still is.
#[test]
fn alembic_and_tests_excluded_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("alembic/versions")).unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::write(
        dir.join("alembic/versions/0001_init.py"),
        "def upgrade():\n    pass\n\n\ndef downgrade():\n    pass\n",
    )
    .unwrap();
    fs::write(
        dir.join("tests/helpers.py"),
        "def make_fixture():\n    return 1\n",
    )
    .unwrap();
    fs::write(dir.join("app.py"), "def really_dead():\n    return 1\n").unwrap();

    let names = [
        "alembic/versions/0001_init.py",
        "tests/helpers.py",
        "app.py",
    ];
    let files: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    let cfg = crate::config::model::Config::default().dead_code; // includes default entry_points
    let dead: Vec<_> = detect(&cg, &files, &cfg)
        .iter()
        .map(|f| f.symbol.clone())
        .collect();

    assert!(
        !dead.contains(&"upgrade".to_string()),
        "alembic excluded by default"
    );
    assert!(
        !dead.contains(&"make_fixture".to_string()),
        "tests excluded by default"
    );
    assert!(
        dead.contains(&"really_dead".to_string()),
        "real dead code still flagged"
    );
}

/// A symbol reached via module attribute access (`crud.fetch_rows()`) is
/// credited to *that* module precisely: `pkg.crud.fetch_rows` is live, but a
/// same-named `fetch_rows` in an unrelated, unimported module stays flagged
/// (proving it's per-binding resolution, not a global name match).
#[test]
fn module_attribute_access_is_credited_precisely() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("pkg")).unwrap();
    fs::write(dir.join("pkg/__init__.py"), "").unwrap();
    fs::write(dir.join("pkg/crud.py"), "def fetch_rows():\n    return 1\n").unwrap();
    // Same symbol name, different module, never imported or accessed.
    fs::write(
        dir.join("pkg/other.py"),
        "def fetch_rows():\n    return 2\n",
    )
    .unwrap();
    fs::write(
        dir.join("pkg/consumer.py"),
        "from pkg import crud\n\n\ndef use():\n    return crud.fetch_rows()\n",
    )
    .unwrap();

    let names = [
        "pkg/__init__.py",
        "pkg/crud.py",
        "pkg/other.py",
        "pkg/consumer.py",
    ];
    let files: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), i as u32).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);
    let dead: Vec<_> = detect(&cg, &files, &cfg())
        .iter()
        .map(|f| (f.module.clone(), f.symbol.clone()))
        .collect();

    assert!(
        !dead.contains(&("pkg.crud".into(), "fetch_rows".into())),
        "crud.fetch_rows is attribute-accessed → live; got {dead:?}"
    );
    assert!(
        dead.contains(&("pkg.other".into(), "fetch_rows".into())),
        "same-named symbol in an unimported module must still be flagged; got {dead:?}"
    );
}

/// An unimported "app" module with a decorated route handler must NOT flag the
/// handler, and a plain unused function there is Low (the module might be an
/// undeclared entry point) — never High.
#[test]
fn decorated_handlers_live_unimported_module_is_low() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("app.py"),
        "import framework\n\napp = framework.App()\n\n\n@app.get(\"/\")\ndef handler():\n    return 1\n\n\ndef plain():\n    return 2\n",
    )
    .unwrap();
    let files = vec![parse_file(&dir.join("app.py"), 0).unwrap()];
    let cg = crate::spine::graph::build(&files, &[]);
    let findings = detect(&cg, &files, &cfg());

    let handler = findings.iter().find(|f| f.symbol == "handler");
    assert!(
        handler.is_none(),
        "decorated route handler must be treated as live"
    );
    let plain = findings
        .iter()
        .find(|f| f.symbol == "plain")
        .expect("plain is unused");
    assert_eq!(
        plain.confidence,
        Confidence::Low,
        "0-inbound module ⇒ Low, not High"
    );
}

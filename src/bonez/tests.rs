use super::*;
use crate::spine::ir::FileId;
use crate::spine::parser::parse_file;
use std::fs;

#[test]
fn flags_forbidden_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("domain")).unwrap();
    fs::create_dir_all(dir.join("web")).unwrap();
    fs::write(dir.join("domain/__init__.py"), "").unwrap();
    fs::write(dir.join("web/__init__.py"), "").unwrap();
    fs::write(dir.join("domain/core.py"), "from web.views import render\n").unwrap();
    fs::write(dir.join("web/views.py"), "def render():\n    pass\n").unwrap();

    let names = [
        "domain/__init__.py",
        "web/__init__.py",
        "domain/core.py",
        "web/views.py",
    ];
    let files: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), FileId::from_index(i)).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);

    // Plain prefix rule (original behavior).
    let prefix = audit(&cg, &[rule("domain", "web")]);
    assert_eq!(prefix.violations.len(), 1);
    assert_eq!(prefix.violations[0].from_module, "domain.core");
    assert!(prefix.unmatched_rules.is_empty());

    // Glob on module name.
    let glob = audit(&cg, &[rule("domain.*", "web.*")]);
    assert_eq!(glob.violations.len(), 1, "glob on module name matches");

    // Glob on file path — robust to module naming.
    let path = audit(&cg, &[rule("**/domain/**", "**/web/**")]);
    assert_eq!(path.violations.len(), 1, "glob on file path matches");

    // A rule that matches no module is reported, not silently "clean".
    let dead = audit(&cg, &[rule("nonexistent.layer", "web")]);
    assert!(dead.violations.is_empty());
    assert_eq!(dead.unmatched_rules.len(), 1, "dead rule surfaced");
}

fn rule(from: &str, to: &str) -> ForbiddenRule {
    ForbiddenRule {
        from: from.into(),
        to: to.into(),
    }
}

/// Regression: a namespace-package layout (no `__init__.py`, no `roots`)
/// where the import target doesn't resolve to an in-repo node is still
/// caught by a path-glob `from` + a dotted-glob `to` (matched against the
/// literal import string).
#[test]
fn matches_namespace_layout_via_path_and_literal_target() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("app/api/endpoints")).unwrap();
    fs::create_dir_all(dir.join("app/db")).unwrap();
    fs::write(
        dir.join("app/api/endpoints/users.py"),
        "from app.db.models import Row\n",
    )
    .unwrap();
    fs::write(dir.join("app/db/models.py"), "class Row:\n    pass\n").unwrap();

    let files: Vec<_> = ["app/api/endpoints/users.py", "app/db/models.py"]
        .iter()
        .enumerate()
        .map(|(i, n)| parse_file(&dir.join(n), FileId::from_index(i)).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&files, &[]);

    // dotted `to` glob matches the literal import string.
    let dotted = audit(&cg, &[rule("**/api/endpoints/**", "app.db.*")]);
    assert_eq!(dotted.violations.len(), 1, "path-from + dotted-to matches");
    assert!(dotted.unmatched_rules.is_empty());

    // path `to` glob matches the dotted import via dot→slash — the rule
    // shape that previously failed on a namespace (no __init__.py) layout.
    let path = audit(&cg, &[rule("**/endpoints/**", "**/db/**")]);
    assert_eq!(
        path.violations.len(),
        1,
        "path-from + path-to matches an unresolved import"
    );
}

fn graph_of(files: &[(&str, &str)]) -> (tempfile::TempDir, CodebaseGraph) {
    let tmp = tempfile::tempdir().unwrap();
    let parsed: Vec<_> = files
        .iter()
        .map(|(name, text)| crate::test_support::write(tmp.path(), name, text))
        .filter(|path| path.extension().is_some_and(|ext| ext != "json"))
        .enumerate()
        .map(|(i, path)| parse_file(&path, FileId::from_index(i)).unwrap())
        .collect();
    let cg = crate::spine::graph::build(&parsed, &[]);
    (tmp, cg)
}

#[test]
fn glob_to_matches_import_with_no_file_in_repo() {
    let (_tmp, cg) = graph_of(&[(
        "app/api/users.py",
        "from vendor.payments.client import Pay\n",
    )]);

    let audit = audit(&cg, &[rule("**/api/**", "vendor.payments.*")]);

    assert_eq!(audit.violations.len(), 1);
    assert_eq!(audit.violations[0].line, 1);
}

#[test]
fn path_glob_from_matching_no_file_is_unmatched() {
    let (_tmp, cg) = graph_of(&[("app/api/users.py", "from app.db import models\n")]);

    let audit = audit(&cg, &[rule("**/services/**", "app.db.*")]);

    assert!(audit.violations.is_empty());
    assert_eq!(audit.unmatched_rules, vec!["**/services/** -x-> app.db.*"]);
}

#[test]
fn path_glob_from_matching_a_file_is_not_unmatched() {
    let (_tmp, cg) = graph_of(&[("app/api/users.py", "import os\n")]);

    let audit = audit(&cg, &[rule("**/api/*.py", "app.db")]);

    assert!(audit.unmatched_rules.is_empty());
}

#[cfg(feature = "lang-typescript")]
#[test]
fn plain_rule_matches_slash_keyed_modules() {
    let (_tmp, cg) = graph_of(&[
        ("package.json", "{}"),
        (
            "src/domain/core.ts",
            "import { send } from '../api/client';\nexport const x = send;\n",
        ),
        ("src/api/client.ts", "export const send = 1;\n"),
    ]);

    let audit = audit(&cg, &[rule("src/domain", "src/api")]);

    assert_eq!(audit.violations.len(), 1);
    assert!(audit.unmatched_rules.is_empty());
}

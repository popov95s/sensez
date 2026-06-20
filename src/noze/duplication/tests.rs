use super::detect;
use super::test_support::write_files;
use crate::config::model::Duplication;
use crate::noze::CloneClass;

/// Config with the given threshold, no excludes, no gap stitching.
fn cfg(threshold: usize) -> Duplication {
    Duplication {
        exclude: vec![],
        threshold,
        max_gap: 0,
        near_miss: false,
        class_property_overlap_min: 4,
    }
}

/// As [`cfg`] but with consistent-rename near-miss detection enabled.
fn cfg_near_miss(threshold: usize) -> Duplication {
    Duplication {
        near_miss: true,
        ..cfg(threshold)
    }
}

fn touches(clones: &[CloneClass], stem: &str) -> bool {
    clones.iter().any(|c| {
        c.occurrences
            .iter()
            .any(|o| o.file.to_string_lossy().contains(stem))
    })
}

/// Verbatim copy (only local variable names differ) → reported. Local renames
/// are allowed; the methods/modules/structure are identical.
#[test]
fn local_rename_copy_is_a_clone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let foo = "def foo(a, b):\n    d = module_a.get(b)\n    e = module_c.get(a)\n    return e\n";
    let bar = "def bar(x, y):\n    f = module_a.get(y)\n    g = module_c.get(x)\n    return g\n";
    let files = write_files(&dir, &[("foo.py", foo), ("bar.py", bar)]);
    let clones = detect(&files, &cfg(10));
    assert!(!clones.is_empty(), "local-renamed copy should be a clone");
}

/// Different methods / different structure → NOT a clone, even though the
/// "shape" (assign-ish, call, return) is similar. `.get` vs `.post`/`.del`.
#[test]
fn different_methods_are_not_a_clone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let foo = "def foo(a, b):\n    d = module_a.get(b)\n    e = module_c.get(a)\n    return f\n";
    let bar = "def bar(a, b):\n    module_a.post(b)\n    module_c.delete(a)\n    return d\n";
    let files = write_files(&dir, &[("foo.py", foo), ("bar.py", bar)]);
    let clones = detect(&files, &cfg(8));
    assert!(
        clones.is_empty(),
        "different methods/structure must not match, got {clones:?}"
    );
}

/// Same method, different LITERAL value → not a clone (literals are kept).
#[test]
fn different_literals_are_not_a_clone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let a = "def a(x):\n    y = module.scale(x, 100)\n    z = module.scale(y, 100)\n    return z\n";
    let b = "def b(x):\n    y = module.scale(x, 250)\n    z = module.scale(y, 250)\n    return z\n";
    let files = write_files(&dir, &[("a.py", a), ("b.py", b)]);
    assert!(
        detect(&files, &cfg(8)).is_empty(),
        "differing literal constants must not match"
    );
}

/// Parameter TYPES are kept, so two signatures that differ only by type don't
/// match (tiny bodies keep the signature as the matched region; an identical
/// *body* would legitimately clone regardless of the signature).
#[test]
fn different_param_types_are_not_a_clone() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let foo = "def foo(a: str, b: str, c: str):\n    return a\n";
    let bar_int = "def bar(x: int, y: int, z: int):\n    return x\n";
    let files = write_files(&dir, &[("foo.py", foo), ("bar.py", bar_int)]);
    assert!(
        detect(&files, &cfg(6)).is_empty(),
        "signatures differing only by type must not match, got {:?}",
        detect(&files, &cfg(6))
    );
    // Same types → the (renamed) signature matches.
    let bar_str = "def bar(x: str, y: str, z: str):\n    return x\n";
    let same = write_files(&dir, &[("foo.py", foo), ("bar.py", bar_str)]);
    assert!(
        !detect(&same, &cfg(6)).is_empty(),
        "same types should match"
    );
}

/// Docstrings are ignored (like comments): two identical functions whose only
/// difference is a reworded docstring are one clone, and it spans the signature
/// (the differing docstring no longer splits the run).
#[test]
fn docstrings_are_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let foo = "def transform(a, b):\n    \"\"\"One-line doc.\"\"\"\n    d = module_a.run(b)\n    e = module_c.run(a)\n    return e\n";
    let bar = "def shape(x, y):\n    \"\"\"A much longer docstring.\n\n    Args:\n        x: first.\n        y: second.\n    \"\"\"\n    f = module_a.run(y)\n    g = module_c.run(x)\n    return g\n";
    let files = write_files(&dir, &[("foo.py", foo), ("bar.py", bar)]);
    // Threshold above the body alone, so a match requires the signature too —
    // only possible if the differing docstring is ignored.
    let clones = detect(&files, &cfg(14));
    assert!(
        !clones.is_empty(),
        "identical code with different docstrings should still clone"
    );
}

/// Consistent-rename clones (`users`↔`orders`, `User`↔`Order`) are caught only
/// with `near_miss` enabled — the strict default keeps them apart.
#[test]
fn near_miss_consistent_rename() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let users = "def fetch_users(db):\n    rows = db.query(\"users\")\n    out = []\n    for row in rows:\n        out.append(User(row.id, row.name))\n    return out\n";
    let orders = "def fetch_orders(db):\n    rows = db.query(\"orders\")\n    out = []\n    for row in rows:\n        out.append(Order(row.id, row.name))\n    return out\n";
    let files = write_files(&dir, &[("u.py", users), ("o.py", orders)]);
    // Strict default: the differing literal "users"/"orders" and type User/Order
    // keep these apart.
    assert!(
        detect(&files, &cfg(10)).is_empty(),
        "strict default must not match consistent renames"
    );
    // Opt-in near-miss: a 1:1 rename maps one onto the other → one clone class.
    let clones = detect(&files, &cfg_near_miss(10));
    assert!(
        touches(&clones, "u.py") && touches(&clones, "o.py"),
        "near-miss must pair the two functions, got {clones:?}"
    );
    assert_eq!(clones[0].hint.as_deref(), Some("consistent-rename clone"));
}

/// Near-miss must not flag short, unrelated functions that merely share a shape
/// (the length threshold filters them out).
#[test]
fn near_miss_ignores_short_unrelated() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let a = "def get_name(self):\n    return self.name\n";
    let b = "def get_age(self):\n    return self.age\n";
    let files = write_files(&dir, &[("a.py", a), ("b.py", b)]);
    assert!(
        detect(&files, &cfg_near_miss(50)).is_empty(),
        "short getters are below threshold and must not match"
    );
}

/// `[duplication] exclude` keeps matching files out of the corpus.
#[test]
fn exclude_globs_drop_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let body =
        "def handler(a, b):\n    d = module_a.run(b)\n    e = module_c.run(a)\n    return e\n";
    let files = write_files(
        &dir,
        &[("tests/test_x.py", body), ("tests/test_y.py", body)],
    );
    // Without excludes: the identical test bodies clone.
    assert!(touches(&detect(&files, &cfg(8)), "test_x.py"));
    // With the tests exclude: nothing reported.
    let excluded = Duplication {
        exclude: vec!["**/tests/**".to_string()],
        threshold: 8,
        max_gap: 0,
        near_miss: false,
        class_property_overlap_min: 4,
    };
    assert!(detect(&files, &excluded).is_empty(), "tests excluded");
}

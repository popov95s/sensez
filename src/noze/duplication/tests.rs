use super::detect;
use super::test_support::{pairs, write_files};
use crate::config::model::Duplication;
use crate::report::CloneClass;

/// Config with the given threshold, no excludes, no gap stitching.
fn cfg(threshold: usize) -> Duplication {
    Duplication {
        exclude: vec![],
        threshold,
        max_gap: 0,
        ..Default::default()
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

#[test]
fn near_miss_over_a_structural_sub_range_is_reported_once() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let users = "def fetch_users(db):\n    rows = db.query(\"users\")\n    out = []\n    for row in rows:\n        out.append(row.id)\n        out.append(row.name)\n    return out\n";
    let orders = "def fetch_orders(db):\n    rows = db.query(\"orders\")\n    out = []\n    for row in rows:\n        out.append(row.id)\n        out.append(row.name)\n    return out\n";
    let files = write_files(&dir, &[("u.py", users), ("o.py", orders)]);
    assert_eq!(detect(&files, &cfg(10)).len(), 1, "structural sub-range");
    let clones = detect(&files, &cfg_near_miss(10));
    assert_eq!(clones.len(), 1, "got {clones:?}");
    assert_eq!(clones[0].detector, crate::report::CloneDetector::Structural);
    assert_eq!(
        clones[0].absorbed[0].detector,
        crate::report::CloneDetector::NearMiss
    );
    assert!(clones[0].occurrences.iter().all(|o| o.start_row == 1));
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
    let files = write_files(&dir, &[("scripts/x.py", body), ("scripts/y.py", body)]);
    // Without excludes: the identical bodies clone.
    assert!(touches(&detect(&files, &cfg(8)), "x.py"));
    // With the scripts exclude: nothing reported.
    let excluded = Duplication {
        exclude: vec!["**/scripts/**".to_string()],
        threshold: 8,
        max_gap: 0,
        ..Default::default()
    };
    assert!(detect(&files, &excluded).is_empty(), "scripts excluded");
}

const SUMMARY_HEAD: &str = "def summarize(orders, limit):\n    totals = {}\n    for order in orders:\n        key = order.customer.id\n        entry = totals.get(key, Entry())\n        entry.count += 1\n        entry.amount += order.total * order.quantity\n        totals[key] = entry\n";
const SUMMARY_SORT: &str = "    ranked = sorted(totals.items(), key=by_amount)\n";
const SUMMARY_REPORT: &str = "    report.write(ranked, header=\"summary\")\n";
const SUMMARY_SHIP: &str =
    "    shipping.notify(ranked, channel=\"ops\")\n    cache.store(ranked, ttl=300)\n";
const SUMMARY_TAIL: &str = "    if len(ranked) > limit:\n        ranked = ranked[:limit]\n    audit.log(\"summarized\", len(ranked))\n    archive.save(ranked, bucket=\"reports\", compress=True)\n    return ranked\n";
const INSERT_METRICS: &str = "    metrics.increment(\"orders.summarized\", len(orders))\n";
const INSERT_TRACE: &str = "    tracing.mark(\"ranked\")\n";

fn original() -> String {
    [
        SUMMARY_HEAD,
        SUMMARY_SORT,
        SUMMARY_REPORT,
        SUMMARY_SHIP,
        SUMMARY_TAIL,
    ]
    .concat()
}

/// Both halves around a one-statement insert are below the threshold, so only
/// seed extraction plus stitching reports the copy.
#[test]
fn one_statement_insert_is_stitched_at_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let original = original();
    let edited = [
        SUMMARY_HEAD,
        SUMMARY_SORT,
        INSERT_METRICS,
        SUMMARY_REPORT,
        SUMMARY_SHIP,
        SUMMARY_TAIL,
    ]
    .concat();
    let files = write_files(tmp.path(), &[("a.py", &original), ("b.py", &edited)]);
    assert!(
        detect(&files, &cfg(50)).is_empty(),
        "each exact half must stay below the threshold"
    );
    let clones = detect(
        &files,
        &Duplication {
            exclude: vec![],
            ..Default::default()
        },
    );
    assert!(pairs(&clones, "a.py", "b.py"), "{clones:?}");
}

#[test]
fn two_small_inserts_chain_into_one_clone() {
    let tmp = tempfile::tempdir().unwrap();
    let original = original();
    let edited = [
        SUMMARY_HEAD,
        INSERT_METRICS,
        SUMMARY_SORT,
        SUMMARY_REPORT,
        SUMMARY_SHIP,
        INSERT_TRACE,
        SUMMARY_TAIL,
    ]
    .concat();
    let files = write_files(tmp.path(), &[("a.py", &original), ("b.py", &edited)]);
    let clones = detect(
        &files,
        &Duplication {
            exclude: vec![],
            ..Default::default()
        },
    );
    let stitched = clones
        .iter()
        .find(|class| class.occurrences.len() == 2)
        .unwrap_or_else(|| panic!("no stitched clone in {clones:?}"));
    let spans: Vec<_> = stitched
        .occurrences
        .iter()
        .map(|o| (o.start_row, o.end_row))
        .collect();
    let whole = [(1, original.lines().count()), (1, edited.lines().count())];
    assert_eq!(spans, whole, "{clones:?}");
}

const IDIOM_FETCH: &str = "    try:\n        payload = client.fetch(url, timeout=30)\n    except TimeoutError:\n        log.warning(\"fetch timed out\", url)\n        raise\n";
const IDIOM_STORE: &str =
    "    for row in payload.rows:\n        store.put(row.key, row.value, ttl=600)\n";
const FILLER_USERS: &str = "    users = directory.lookup(payload.owner)\n    if users.active:\n        mailer.send(users.email, subject=\"welcome\")\n    audit.touch(users.id)\n    cache.drop(users.id)\n";
const FILLER_LEDGER: &str = "    total = sum(item.price for item in payload.items)\n    ledger.record(total, currency=\"EUR\")\n    ledger.flush(batch=True)\n    ledger.close()\n";

/// Two unrelated functions sharing two short idioms: the gap outweighs the
/// matched tokens, so the density guard refuses to stitch them.
#[test]
fn sparse_shared_idioms_are_not_stitched() {
    let tmp = tempfile::tempdir().unwrap();
    let users = [
        "def load_users(url):\n",
        IDIOM_FETCH,
        FILLER_USERS,
        IDIOM_STORE,
    ]
    .concat();
    let ledger = [
        "def load_ledger(url):\n",
        IDIOM_FETCH,
        FILLER_LEDGER,
        IDIOM_STORE,
    ]
    .concat();
    let files = write_files(tmp.path(), &[("users.py", &users), ("ledger.py", &ledger)]);
    let config = Duplication {
        max_gap: 25,
        ..cfg(24)
    };
    let clones = detect(&files, &config);
    assert!(clones.is_empty(), "{clones:?}");
}

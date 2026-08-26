use super::{analyze_path_in_session, session::AnalysisSession};
use std::fs;

#[test]
fn changed_file_can_create_and_remove_cross_file_duplication() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(dir.join("sensez.toml"), "[duplication]\nthreshold = 8\n").unwrap();
    let left = "def left(value):\n    first = module.fetch(value)\n    second = module.clean(first)\n    return second\n";
    let unique = "def right(value):\n    return value + 1\n";
    let session = AnalysisSession::default();
    fs::write(dir.join("left.py"), left).unwrap();
    fs::write(dir.join("right.py"), unique).unwrap();

    assert!(analyze_path_in_session(&session, dir, None)
        .unwrap()
        .0
        .duplication
        .is_empty());
    fs::write(dir.join("right.py"), left.replace("left", "right")).unwrap();
    let created = analyze_path_in_session(&session, dir, None).unwrap().0;
    assert!(created.duplication.iter().any(|clone| {
        clone
            .occurrences
            .iter()
            .any(|item| item.file.ends_with("left.py"))
            && clone
                .occurrences
                .iter()
                .any(|item| item.file.ends_with("right.py"))
    }));

    fs::write(dir.join("right.py"), unique).unwrap();
    assert!(analyze_path_in_session(&session, dir, None)
        .unwrap()
        .0
        .duplication
        .is_empty());
}

#[test]
fn changed_import_updates_cross_file_cycle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let session = AnalysisSession::default();
    fs::write(
        dir.join("a.py"),
        "from b import value\n\ndef a():\n    return value\n",
    )
    .unwrap();
    fs::write(dir.join("b.py"), "def value():\n    return 1\n").unwrap();
    assert!(analyze_path_in_session(&session, dir, None)
        .unwrap()
        .0
        .cycles
        .is_empty());

    fs::write(
        dir.join("b.py"),
        "from a import a\n\ndef value():\n    return a()\n",
    )
    .unwrap();
    assert_eq!(
        analyze_path_in_session(&session, dir, None)
            .unwrap()
            .0
            .cycles
            .len(),
        1
    );
}

#[test]
fn cycles_exclude_is_independent_of_smells_exclude() {
    // `a -> b` plus `b -> a` forms the loop.
    let importer = "from b import value\n\ndef a():\n    return value\n";
    let cycle_b = "from a import a\n\ndef value():\n    return a()\n";
    let build = |toml: &str| {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::write(dir.join("sensez.toml"), toml).unwrap();
        fs::write(dir.join("a.py"), importer).unwrap();
        fs::write(dir.join("b.py"), "def value():\n    return 1\n").unwrap();
        let session = AnalysisSession::default();
        // Introduce the back-edge so the loop exists before the first scan.
        fs::write(dir.join("b.py"), cycle_b).unwrap();
        (tmp, session, dir)
    };

    // Dedicated `[cycles] exclude` suppresses the finding. An SCC stays
    // reportable while any member is outside the globs, so suppressing this
    // two-module loop takes both.
    let (tmp, session, dir) =
        build("[cycles]\nexclude = [\"**/a.py\", \"**/b.py\"]\n");
    assert!(analyze_path_in_session(&session, &dir, None)
        .unwrap()
        .0
        .cycles
        .is_empty());
    drop(tmp);

    // A smells-only exclude must NOT hide the cycle anymore.
    let (_tmp, session, dir) = build("[smells]\nexclude = [\"**/b.py\"]\n");
    assert_eq!(
        analyze_path_in_session(&session, &dir, None)
            .unwrap()
            .0
            .cycles
            .len(),
        1,
        "smell excludes must not leak into cycle detection"
    );
}

#[test]
fn changed_consumer_updates_provider_dead_code() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let session = AnalysisSession::default();
    fs::write(
        dir.join("provider.py"),
        "def live():\n    return 1\n\ndef other():\n    return 2\n",
    )
    .unwrap();
    let consumer = dir.join("consumer.py");
    fs::write(&consumer, "from provider import live\n\nprint(live())\n").unwrap();
    assert!(!analyze_path_in_session(&session, dir, None)
        .unwrap()
        .0
        .dead_code
        .iter()
        .any(|finding| finding.symbol == "live"));

    fs::write(&consumer, "from provider import other\n\nprint(other())\n").unwrap();
    assert!(analyze_path_in_session(&session, dir, None)
        .unwrap()
        .0
        .dead_code
        .iter()
        .any(|finding| finding.symbol == "live"));
}

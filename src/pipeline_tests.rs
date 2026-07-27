use super::*;
use crate::diff::ChangedLines;
use std::fs;

#[test]
fn scan_produces_json() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("m.py"), "def f():\n    pass\n").unwrap();

    let json = scan(&dir, Some(50), Format::Json, 0).unwrap();
    assert!(json.contains("\"duplication\""));
}

#[test]
fn diff_mode_filters_to_touched_files() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    // One imported module with an unreferenced top-level function, plus an
    // untouched module. The import gives the touched module enough usage
    // evidence to report its unused symbol.
    fs::write(
        dir.join("touched.py"),
        "def used_here():\n    return 0\n\n\ndef unused_here():\n    return 1\n",
    )
    .unwrap();
    fs::write(
        dir.join("other.py"),
        "def unused_elsewhere():\n    return 2\n",
    )
    .unwrap();
    fs::write(
        dir.join("consumer.py"),
        "from touched import used_here\n\nprint(used_here())\n",
    )
    .unwrap();

    // Pretend only touched.py (its def line) was changed.
    let mut changed = ChangedLines::default();
    changed.add(&dir.join("touched.py"), 5, 6);

    let (report, module_files) = analyze_path(&dir, None).unwrap();
    let mut report = report;
    crate::diff::apply(&mut report, &changed, &module_files);
    assert_eq!(report.meta.mode, crate::report::ReportMode::Diff);
    let dead: Vec<_> = report.dead_code.iter().map(|f| f.symbol.as_str()).collect();
    assert!(dead.contains(&"unused_here"), "touched file's finding kept");
    assert!(
        !dead.contains(&"unused_elsewhere"),
        "untouched file's finding dropped"
    );
    assert!(report
        .dead_code
        .iter()
        .all(|f| f.reason == "added_unreferenced"));
}

/// `--diff` scopes smells by the symbol's BODY span, not just its `def`
/// line: editing code inside a function (even one you didn't originally
/// write) surfaces its smell, while an edit entirely outside it does not.
/// Pinned on synthetic code (NOT from any real repo).
#[test]
fn diff_smell_scoping_covers_the_function_body() {
    use crate::report::SmellKind;

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    // `split_variable` is opt-in; enable just it to keep the report focused.
    fs::write(dir.join("sensez.toml"), "[smells]\nsplit_variable = true\n").unwrap();
    // `result` is plainly reassigned 3x (default threshold) -> SplitVariable
    // on `proc` (def line 1, body lines 2-5). `untouched` is a second fn so
    // an "elsewhere" edit has somewhere to land.
    let src = "def proc(data):\n    \
               result = []\n    \
               result = step_one(data)\n    \
               result = step_two(result)\n    \
               return result\n\n\
               def untouched():\n    \
               return 0\n";
    let file = dir.join("m.py");
    fs::write(&file, src).unwrap();

    let has_split = |changed: &ChangedLines| {
        let (mut report, module_files) = analyze_path(&dir, None).unwrap();
        crate::diff::apply(&mut report, changed, &module_files);
        report
            .smells
            .iter()
            .any(|s| s.kind == SmellKind::SplitVariable && s.symbol == "proc")
    };

    // Sanity: a full scan finds it (proves the fixture triggers the smell).
    assert!(
        analyze_path(&dir, None)
            .unwrap()
            .0
            .smells
            .iter()
            .any(|s| s.kind == SmellKind::SplitVariable),
        "fixture must produce a split_variable smell"
    );

    // Body-only edit (line 4, inside proc but not its def): now SURFACED.
    let mut body_only = ChangedLines::default();
    body_only.add(&file, 4, 4);
    assert!(
        has_split(&body_only),
        "editing the function body must surface its smell"
    );

    // Edit entirely outside proc (the other function): not relevant.
    let mut elsewhere = ChangedLines::default();
    elsewhere.add(&file, 8, 8);
    assert!(
        !has_split(&elsewhere),
        "an edit outside the function must NOT surface its smell"
    );
}

#[test]
fn parse_failures_surface_concrete_scan_issues() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    let deep = format!("x = {}1{}", "(".repeat(100_000), ")".repeat(100_000));
    fs::write(dir.join("too_deep.py"), deep).unwrap();

    let (report, _) = analyze_path(&dir, None).unwrap();
    assert_eq!(report.meta.files_skipped, 1);
    assert_eq!(report.meta.issues.len(), 1);
    assert_eq!(report.meta.issues[0].stage, crate::report::ScanStage::Parse);
    assert!(
        report.meta.issues[0]
            .message
            .contains("syntax tree deeper than"),
        "{:?}",
        report.meta.issues[0]
    );
}

#[test]
fn duplicate_module_names_stay_out_of_scan_issues() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(dir.join("app")).unwrap();
    fs::write(dir.join("app.py"), "def flat():\n    return 1\n").unwrap();
    fs::write(dir.join("app/__init__.py"), "def pkg():\n    return 2\n").unwrap();

    let (report, _) = analyze_path(&dir, None).unwrap();

    assert_eq!(report.meta.files_skipped, 0);
    assert!(report.meta.issues.is_empty());
    assert!(!reporter::to_json(&report)
        .unwrap()
        .contains("already defined"));
    let json = scan(&dir, None, Format::Json, 0).unwrap();
    let terminal = scan(&dir, None, Format::Terminal, 0).unwrap();
    assert!(!json.contains("\"issues\""));
    assert!(!json.contains("already defined"));
    assert!(!terminal.contains("scan issue"));
    assert!(!terminal.contains("already defined"));
}

#[test]
fn action_policy_is_applied_to_pillars_and_smells() {
    use crate::report::SmellKind;

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("sensez.toml"),
        "[action]\ndead_code = \"info\"\n\
         [smells.rules.long_function]\nmax_lines = 2\naction = \"must_fix\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("m.py"),
        "def live():\n    return 0\n\n\ndef unused_long():\n    x = 1\n    y = 2\n    z = 3\n    return x + y + z\n",
    )
    .unwrap();
    fs::write(dir.join("consumer.py"), "from m import live\n\nlive()\n").unwrap();

    let (report, _) = analyze_path(&dir, None).unwrap();
    let dead = report
        .dead_code
        .iter()
        .find(|finding| finding.symbol == "unused_long")
        .expect("unused function should be reported");
    assert_eq!(dead.action, crate::report::ActionLevel::Info);

    let smell = report
        .smells
        .iter()
        .find(|finding| finding.kind == SmellKind::LongFunction)
        .expect("long function should be reported");
    assert_eq!(smell.action, crate::report::ActionLevel::MustFix);
}

use super::*;
use std::fs;

#[test]
fn payload_round_trip_keeps_fields_the_report_serialization_skips() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(
        dir.join("a.py"),
        "def many(a, b, c, d, e, f, g, h, i):\n    if a:\n        if b:\n            if c:\n                if d:\n                    if e:\n                        return f + g + h + i\n    return 0\n",
    )
    .unwrap();
    fs::write(dir.join("b.py"), "import a\n\ndef b():\n    return 2\n").unwrap();
    let (report, module_files) = crate::analyze_path(dir, None).unwrap();
    assert!(!report.smells.is_empty());

    let bytes =
        serde_json::to_vec(&Payload::from_scan(report.clone(), module_files.clone())).unwrap();
    let (restored, restored_files) = serde_json::from_slice::<Payload>(&bytes)
        .unwrap()
        .into_scan();

    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        serde_json::to_value(&report).unwrap()
    );
    assert_eq!(restored_files, module_files);
    let spans = |report: &AnalysisReport| -> Vec<(usize, Severity)> {
        report
            .smells
            .iter()
            .map(|smell| (smell.end_line, smell.severity))
            .collect()
    };
    assert_eq!(spans(&restored), spans(&report));
}

#[test]
fn a_child_that_cannot_start_falls_back_to_an_in_process_scan() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("no-such-binary");

    let scan = analyze(&missing, tmp.path(), None).unwrap();

    assert!(scan.is_none());
}

#[test]
fn child_scans_through_the_parse_cache_match_a_fresh_scan() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    fs::write(dir.join("a.py"), "def unused():\n    return 1\n").unwrap();
    fs::write(
        dir.join("b.ts"),
        "export const used = 1;\nfunction orphan() {}\n",
    )
    .unwrap();
    let fresh = serde_json::to_value(crate::analyze_path(dir, None).unwrap().0).unwrap();

    let cold = crate::pipeline::analyze_path_for_child(dir, None)
        .unwrap()
        .0;
    let warm = crate::pipeline::analyze_path_for_child(dir, None)
        .unwrap()
        .0;

    assert!(dir.join(".sensez/parse-cache").is_dir());
    assert_eq!(serde_json::to_value(cold).unwrap(), fresh);
    assert_eq!(serde_json::to_value(warm).unwrap(), fresh);
}

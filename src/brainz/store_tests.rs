use super::events::{Event, Totals};
use super::fingerprint::{Aged, ResolvedHistory};
use super::store::*;
use std::collections::BTreeMap;
use std::fs;

#[test]
fn totals_and_events_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    fs::create_dir_all(&root).unwrap();

    assert_eq!(load_totals(&root).scans, 0, "missing file -> defaults");

    let mut totals = Totals::default();
    let event = Event::Outcome {
        ts: 1,
        session: "s".into(),
        branch: "main".into(),
        pillar: "smells".into(),
        action: "fixed".into(),
        count: 1,
        detail: Some("renamed god module".into()),
    };
    totals.absorb(&event);
    save_totals(&root, &totals).unwrap();
    append_events(&root, std::slice::from_ref(&event)).unwrap();
    append_events(&root, &[event]).unwrap();

    assert_eq!(load_totals(&root).outcomes["fixed:smells"], 1);
    let log = fs::read_to_string(root.join(".sensez/local-metrics/events.jsonl")).unwrap();
    assert_eq!(log.lines().count(), 2, "appends accumulate");

    let prints: Aged = BTreeMap::from([(
        "dead_code".into(),
        BTreeMap::from([(
            "7".to_string(),
            super::fingerprint::AgedEntry {
                first_seen: 1,
                label: "x".into(),
                detector: "dead_code/function".into(),
            },
        )]),
    )]);
    let history: ResolvedHistory = BTreeMap::from([(
        "dead".to_string(),
        super::fingerprint::ResolvedRecord {
            detector: "dead_code/function".into(),
            resolved_ts: 5,
        },
    )]);
    save_fingerprints(&root, "main", &prints, &history, 10).unwrap();
    assert_eq!(load_fingerprints(&root, "main"), prints);
    assert_eq!(load_resolved_history(&root, "main"), history);
}

#[test]
fn fingerprints_are_isolated_per_branch() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let prints: Aged = BTreeMap::from([(
        "dead_code".into(),
        BTreeMap::from([(
            "abc".to_string(),
            super::fingerprint::AgedEntry {
                first_seen: 1,
                label: "main-only".into(),
                detector: "dead_code/function".into(),
            },
        )]),
    )]);
    save_fingerprints(&root, "main", &prints, &ResolvedHistory::new(), 1).unwrap();
    assert_eq!(load_fingerprints(&root, "main"), prints);
    assert!(
        load_fingerprints(&root, "feature").is_empty(),
        "a different branch must not see main's baseline"
    );
}

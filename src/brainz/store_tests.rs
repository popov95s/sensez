use super::events::{Event, OutcomeKey, Totals};
use super::fingerprint::{Aged, AgedEntry, Detector, Label, Namespace, ResolvedHistory};
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

    assert_eq!(
        load_totals(&root).outcomes[&OutcomeKey::new("fixed", "smells")],
        1
    );
    let log = fs::read_to_string(root.join(".sensez/local-metrics/events.jsonl")).unwrap();
    assert_eq!(log.lines().count(), 2, "appends accumulate");

    let prints: Aged = BTreeMap::from([(
        Namespace::DeadCode,
        BTreeMap::from([(
            "7".to_string(),
            AgedEntry {
                first_seen: 1,
                label: dead_label("x"),
                class: dead_function(),
            },
        )]),
    )]);
    let history: ResolvedHistory = BTreeMap::from([(
        "dead".to_string(),
        super::fingerprint::ResolvedRecord {
            class: dead_function(),
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
        Namespace::DeadCode,
        BTreeMap::from([(
            "abc".to_string(),
            AgedEntry {
                first_seen: 1,
                label: dead_label("main-only"),
                class: dead_function(),
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

fn dead_function() -> Detector {
    Detector::DeadCode {
        symbol_kind: "function".to_string(),
    }
}

fn dead_label(symbol: &str) -> Label {
    Label::DeadCode {
        module: "app".to_string(),
        symbol: symbol.to_string(),
        symbol_kind: "function".to_string(),
    }
}

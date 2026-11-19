use super::*;
use crate::brainz::fingerprint::{Aged, AgedEntry, Label};
use std::fs;

#[test]
fn mark_matches_by_label_and_clears() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    fs::create_dir_all(&root).unwrap();

    let aged: Aged = BTreeMap::from([(
        Pillar::DeadCode,
        BTreeMap::from([(
            "abc1".to_string(),
            AgedEntry {
                first_seen: 1,
                label: Label::DeadCode {
                    module: "app".to_string(),
                    symbol: "orphan".to_string(),
                    symbol_kind: "function".to_string(),
                },
                class: Detector::DeadCode {
                    symbol_kind: "function".to_string(),
                },
            },
        )]),
    )]);
    store::save_fingerprints(&root, "main", &aged, &Default::default(), 1).unwrap();

    assert!(mark(
        &root,
        "main",
        Pillar::DeadCode,
        "nomatch",
        TriageVerdict::Debt,
        None
    )
    .is_err());

    let hit = mark(
        &root,
        "main",
        Pillar::DeadCode,
        "ORPHAN",
        TriageVerdict::FalsePositive,
        Some("dynamic dispatch".into()),
    )
    .unwrap();
    assert_eq!(
        hit,
        vec![Marked {
            label: "app::orphan (function)".to_string(),
            detector: Detector::DeadCode {
                symbol_kind: "function".to_string(),
            },
        }]
    );
    let triage = load(&root);
    assert_eq!(triage["abc1"].verdict, TriageVerdict::FalsePositive);
    assert!(ignored_keys(&triage).contains("abc1"));

    mark(
        &root,
        "main",
        Pillar::DeadCode,
        "orphan",
        TriageVerdict::Clear,
        None,
    )
    .unwrap();
    assert!(load(&root).is_empty());
}

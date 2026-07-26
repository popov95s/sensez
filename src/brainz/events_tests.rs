use super::*;

#[test]
fn absorb_aggregates_each_event_kind() {
    let mut t = Totals::default();
    t.absorb(&Event::Scan {
        ts: 10,
        session: "s".into(),
        branch: "main".into(),
        ms: 5,
        origin: Origin::Gate,
        reported: BTreeMap::from([("dead_code/function".into(), 3)]),
        resolved: BTreeMap::from([(
            "dead_code/function".into(),
            Resolved {
                count: 1,
                secs_total: 3600,
            },
        )]),
        reintroduced: BTreeMap::from([(
            "smells/god_module".into(),
            Resolved {
                count: 2,
                secs_total: 200,
            },
        )]),
        files: 120,
        loc: 9000,
        config_hash: Some(42),
    });
    t.absorb(&Event::Search {
        ts: 20,
        session: "s".into(),
        branch: "main".into(),
        ms: 5,
        query_len: 12,
        hits: 4,
        top_score: 0.7,
        first_on_repo: true,
        bytes_returned: 100,
        file_bytes_referenced: 5000,
    });
    t.absorb(&Event::Outcome {
        ts: 30,
        session: "s".into(),
        branch: "main".into(),
        pillar: "duplication".into(),
        action: "fixed".into(),
        count: 2,
        detail: None,
    });

    assert_eq!((t.first_used, t.last_used), (10, 30));
    assert_eq!((t.scans, t.searches, t.first_searches), (1, 1, 1));
    assert_eq!(t.outcomes[&OutcomeKey::new("fixed", "duplication")], 2);
    assert_eq!(t.est_context_bytes_saved, 4900);
    assert_eq!(t.scans_by_origin["gate"], 1);
    assert_eq!(t.reported_by_detector["dead_code/function"], 3);
    let ttr = &t.resolved_by_detector["dead_code/function"];
    assert_eq!((ttr.count, ttr.secs_total), (1, 3600));
    assert_eq!(t.reintroduced_by_detector["smells/god_module"].count, 2);
    assert_eq!(
        (t.scan_ms_total, t.scan_files_total, t.scan_loc_total),
        (5, 120, 9000)
    );
    // One scan → a config hash is anchored but no change is counted yet.
    assert_eq!((t.config_changes, t.last_config_hash), (0, Some(42)));
}

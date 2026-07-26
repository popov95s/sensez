use super::*;

fn totals() -> Totals {
    let mut t = Totals::default();
    // dead_code/function: 3 fixed, 1 deferred (debt), 0 wrong → precision 1.0.
    t.resolved_by_detector.insert(
        "dead_code/function".into(),
        Resolved {
            count: 3,
            secs_total: 3 * 86_400,
        },
    );
    t.outcomes
        .insert(OutcomeKey::new("debt", "dead_code/function"), 1);
    // smells/god_module: 1 fixed, 3 false positives → precision 0.25.
    t.resolved_by_detector.insert(
        "smells/god_module".into(),
        Resolved {
            count: 1,
            secs_total: 86_400,
        },
    );
    t.outcomes
        .insert(OutcomeKey::new("false_positive", "smells/god_module"), 3);
    t.scans_by_origin.insert("gate".into(), 3);
    t.scans_by_origin.insert("tool".into(), 1);
    t.searches = 4;
    t.searches_zero_hit = 1;
    // 2 of dead_code/function's 3 fixes came back → reintroduction rate 0.67,
    // each having held for 5 days on average (2 * 5 days summed).
    t.reintroduced_by_detector.insert(
        "dead_code/function".into(),
        Resolved {
            count: 2,
            secs_total: 2 * 5 * 86_400,
        },
    );
    t.scans = 4;
    t.scan_ms_total = 800;
    t.scan_files_total = 400;
    t.scan_loc_total = 8000;
    t.config_changes = 2;
    t
}

#[test]
fn precision_blends_fixes_debt_and_false_positives() {
    let p = precision_by_detector(&totals());
    assert_eq!(p["dead_code/function"]["precision"], 1.0);
    assert_eq!(p["dead_code/function"]["resolved"], 3);
    assert_eq!(p["smells/god_module"]["precision"], 0.25);
    assert_eq!(p["smells/god_module"]["false_positive"], 3);
}

#[test]
fn gate_and_search_funnels_are_ratios() {
    let t = totals();
    assert_eq!(gate_funnel(&t)["gate_share"], 0.75);
    assert_eq!(search_health(&t)["zero_hit_rate"], 0.25);
}

#[test]
fn mean_resolution_days_rolls_up() {
    let ttr = mean_resolution_days(&totals().resolved_by_detector);
    assert_eq!(ttr["dead_code/function"], 1.0);
    // 4 findings over (3+1) days total → 1.0 day overall.
    assert_eq!(ttr["_overall"], 1.0);
}

#[test]
fn fix_reintroductions_pair_reintroductions_with_fixes() {
    let r = fix_reintroductions_by_detector(&totals());
    assert_eq!(r["dead_code/function"]["reintroduced"], 2);
    assert_eq!(r["dead_code/function"]["resolved"], 3);
    assert_eq!(r["dead_code/function"]["rate"], 0.67);
    assert_eq!(r["dead_code/function"]["mean_days_until_reintroduced"], 5.0);
    // god_module had no reintroductions → absent.
    assert!(r.get("smells/god_module").is_none());
}

#[test]
fn self_health_reports_throughput() {
    let h = self_health(&totals());
    assert_eq!(h["mean_scan_ms"], 200.0); // 800ms / 4 scans
    assert_eq!(h["ms_per_kfile"], 2000.0); // 800ms * 1000 / 400 files
    assert_eq!(h["ms_per_kloc"], 100.0); // 800ms * 1000 / 8000 loc
}

#[test]
fn low_precision_detectors_need_evidence_and_a_floor() {
    let noisy = low_precision_detectors(&totals());
    // god_module: 1 real / 4 adjudicated = 0.25 < floor, 4 samples → noisy.
    assert!(noisy.contains("smells/god_module"));
    // dead_code/function: 4 real / 4 = 1.0 → trusted.
    assert!(!noisy.contains("dead_code/function"));

    // One stray false positive is not enough evidence to brand a detector.
    let mut t = Totals::default();
    t.outcomes
        .insert(OutcomeKey::new("false_positive", "smells/x"), 1);
    assert!(low_precision_detectors(&t).is_empty());
}

#[test]
fn gate_conversion_counts_escapes() {
    let blocked: BTreeSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
    let open: BTreeSet<String> = ["b"].iter().map(|s| s.to_string()).collect();
    let c = gate_conversion(&blocked, &open, true);
    assert_eq!(c["blocked_findings"], 3);
    assert_eq!(c["still_open"], 1);
    assert_eq!(c["resolved"], 2);
    assert_eq!(c["conversion_rate"], 0.67);

    // No baseline → rate is null (not a false 1.0), even though `open` is empty.
    let none = gate_conversion(&blocked, &BTreeSet::new(), false);
    assert_eq!(none["conversion_rate"], Value::Null);
    assert_eq!(none["blocked_findings"], 3);
}

#[test]
fn calibration_flags_noise_and_hotspots() {
    let tips = calibration_suggestions(&totals(), &crate::config::model::Config::default());
    let arr = tips.as_array().unwrap();
    assert!(arr
        .iter()
        .any(|t| t.as_str().unwrap().contains("smells/god_module")));
    assert!(arr
        .iter()
        .any(|t| t.as_str().unwrap().contains("dead_code/function")
            && t.as_str().unwrap().contains("regressed")));
}

#[test]
fn config_pressure_counts_changes_and_scope() {
    let cfg = crate::config::model::Config {
        exclude: vec!["**/tests/**".into(), "**/gen/**".into()],
        ..Default::default()
    };
    let p = config_pressure(&totals(), &cfg);
    assert_eq!(p["changes"], 2);
    assert_eq!(p["scope"]["exclude_globs"], 2);
}

#[test]
fn empty_totals_yield_empty_or_zero() {
    let t = Totals::default();
    assert_eq!(precision_by_detector(&t), json!({}));
    assert_eq!(mean_resolution_days(&t.resolved_by_detector), json!({}));
    assert_eq!(fix_reintroductions_by_detector(&t), json!({}));
    assert_eq!(search_health(&t)["zero_hit_rate"], 0.0);
    assert_eq!(self_health(&t)["ms_per_kfile"], 0.0);
}

use super::*;
use crate::brainz::fingerprint::{fingerprints, ResolvedHistory};
use crate::brainz::staleness::STALE_AFTER_SECS;
use serde_json::json;

#[test]
fn age_tracks_resolved_staleness_and_triage_exclusions() {
    let first = fingerprints(&json!({
        "dead_code": [
            {"module": "a", "symbol": "f", "kind": "function", "line": 10},
            {"module": "a", "symbol": "g", "kind": "function", "line": 20},
        ],
        "smells": [{"kind": "long_function", "file": "a.py", "symbol": "f", "metric": 80}],
    }));
    let none = HashSet::new();
    let empty_hist = ResolvedHistory::new();
    let first_age = age(&Aged::new(), &first, &empty_hist, 1_000, &none);
    assert!(first_age.resolved.is_empty(), "first scan: nothing to diff");

    let later = 1_000 + STALE_AFTER_SECS + 86_401;
    let current = fingerprints(&json!({
        "dead_code": [{"module": "a", "symbol": "f", "kind": "function", "line": 99}],
        "smells": [{"kind": "long_function", "file": "a.py", "symbol": "f", "metric": 75}],
    }));
    let a = age(&first_age.aged, &current, &first_age.history, later, &none);
    let dead = a.resolved.get("dead_code/function").expect("g resolved");
    assert_eq!(dead.count, 1);
    assert_eq!(
        dead.secs_total,
        later - 1_000,
        "time-to-resolution = now - first_seen"
    );
    assert_eq!(
        a.resolved.get("smells/long_function"),
        None,
        "line/metric drift is not a fix"
    );
    let entries = crate::brainz::staleness::stale_entries(&a.aged, later, &none);
    assert_eq!(entries.len(), 2);
    assert!(entries
        .iter()
        .any(|(p, _, label, days)| p == "dead_code" && label == "a::f (function)" && *days >= 8));

    let fp_key = entries
        .iter()
        .find(|(p, ..)| p == "dead_code")
        .map(|(_, k, ..)| k.clone())
        .unwrap();
    let ignore: HashSet<String> = [fp_key].into();
    assert_eq!(
        crate::brainz::staleness::stale_entries(&a.aged, later, &ignore).len(),
        1
    );
    let empty = fingerprints(&json!({"dead_code": [], "smells": []}));
    let b = age(&a.aged, &empty, &a.history, later + 1, &ignore);
    assert_eq!(
        b.resolved.get("dead_code/function"),
        None,
        "vanished FP is not value"
    );
    assert_eq!(
        b.resolved.get("smells/long_function").map(|r| r.count),
        Some(1),
        "real finding fixed counts"
    );
}

#[test]
fn reintroduced_finding_is_detected_and_history_expires() {
    let none = HashSet::new();
    let dc = |line| {
        fingerprints(&json!({
            "dead_code": [{"module": "a", "symbol": "f", "kind": "function", "line": line}],
        }))
    };
    let seen = age(&Aged::new(), &dc(10), &ResolvedHistory::new(), 100, &none);
    let gone = age(
        &seen.aged,
        &fingerprints(&json!({"dead_code": []})),
        &seen.history,
        200,
        &none,
    );
    assert_eq!(gone.resolved["dead_code/function"].count, 1);
    assert_eq!(gone.history.len(), 1, "resolved fingerprint is remembered");

    let back = age(&gone.aged, &dc(42), &gone.history, 300, &none);
    let r = back
        .reintroduced
        .get("dead_code/function")
        .expect("reintro");
    assert_eq!((r.count, r.secs_total), (1, 100));
    assert!(back.history.is_empty(), "reappearance clears the record");

    // A reappearance beyond the window is NOT a reintroduction (record expired).
    let stale_back = age(
        &gone.aged,
        &dc(42),
        &gone.history,
        200 + REINTRO_WINDOW_SECS + 1,
        &none,
    );
    assert!(
        stale_back.reintroduced.is_empty(),
        "expired record is forgotten"
    );
}

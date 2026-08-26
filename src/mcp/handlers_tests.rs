use super::super::protocol::handle_message;
use super::ScanArgs;
use crate::test_support::GitTestRepo;
use serde_json::{json, Value};

#[test]
fn tools_list_includes_metrics_tools() {
    let _metrics = crate::test_support::metrics_guard();
    let req = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list"});
    let resp = handle_message(&req).unwrap();
    let names: Vec<&str> = resp["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"noze_sniff"));
    assert!(names.contains(&"get_configuration_summary"));
    assert!(names.contains(&"brainz_triage"));
    assert!(names.contains(&"brainz_report"));
    assert!(!names.contains(&"record_outcome"));
}

#[test]
fn usage_report_serves_a_clean_repo() {
    let _metrics = crate::test_support::metrics_guard();
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.to_string_lossy().into_owned();

    let req = json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {
        "name": "brainz_report", "arguments": {"path": path}
    }});
    let resp = handle_message(&req).unwrap();
    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let report: Value = serde_json::from_str(text).unwrap();
    assert!(report.get("session").is_some() && report.get("all_time").is_some());
}

#[test]
fn scan_tool_omits_duplicate_module_noise() {
    let _metrics = crate::test_support::metrics_guard();
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::write(dir.join("app.py"), "def flat():\n    return 1\n").unwrap();
    std::fs::write(dir.join("app/__init__.py"), "def pkg():\n    return 2\n").unwrap();
    let path = dir.to_string_lossy().into_owned();

    let req = json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {
        "name": "noze_sniff", "arguments": {"path": path}
    }});
    let resp = handle_message(&req).unwrap();

    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.contains("already defined"));
    assert!(!text.contains("\"issues\""));
    assert!(!text.contains("_total"));
    assert!(!text.contains("\"analyzed_files\""));
    assert!(!text.contains("\"internal_edges\""));
    assert!(!text.contains("\"external_edges\""));
    assert!(!text.contains("\"source_lines\""));
}

#[test]
fn scan_tool_omits_scan_diagnostics() {
    let _metrics = crate::test_support::metrics_guard();
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let deep = format!("x = {}1{}", "(".repeat(100_000), ")".repeat(100_000));
    std::fs::write(dir.join("too_deep.py"), deep).unwrap();
    let path = dir.to_string_lossy().into_owned();

    let req = json!({"jsonrpc": "2.0", "id": 10, "method": "tools/call", "params": {
        "name": "noze_sniff", "arguments": {"path": path}
    }});
    let resp = handle_message(&req).unwrap();

    assert_eq!(resp["result"]["isError"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.contains("\"issues\""));
    assert!(!text.contains("\"files_skipped\": 1"));
    assert!(!text.contains("syntax tree deeper than"));
}

#[test]
fn scan_args_defaults_to_diff_mode() {
    let _metrics = crate::test_support::metrics_guard();
    // The typed struct is the schema — defaults live in `Default` impl,
    // not buried in a `Value::get` chain. Empty args, explicit
    // `true`, and explicit `false` all deserialize to the right
    // `diff`/`record` booleans.
    let from_empty: ScanArgs = serde_json::from_value(json!({})).unwrap();
    assert!(from_empty.path.is_empty());
    assert_eq!(from_empty.threshold, None);
    assert_eq!(from_empty.limit, 0);
    assert!(from_empty.diff);
    assert!(from_empty.record);

    let from_true: ScanArgs = serde_json::from_value(json!({"diff": true})).unwrap();
    assert!(from_true.diff);

    let from_false: ScanArgs = serde_json::from_value(json!({"diff": false})).unwrap();
    assert!(!from_false.diff);
}

#[test]
fn diff_scan_refreshes_metrics_baseline() {
    let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("m.py") else {
        return;
    };
    std::fs::write(&repo.file, "def f():\n    pass\n").unwrap();

    let req = json!({"jsonrpc": "2.0", "id": 11, "method": "tools/call", "params": {
        "name": "noze_sniff", "arguments": {"path": repo.path, "diff": true}
    }});
    let resp = handle_message(&req).unwrap();

    assert_eq!(resp["result"]["isError"], false);
    assert!(std::path::Path::new(&repo.path)
        .join(".sensez/local-metrics/last-scan.json")
        .exists());
}

/// `noze_sniff` must land a Scan event in brainz with the report's
/// detector counts. Without this, the precision/fix reintroduction signals
/// would be starved of their numerator.
#[test]
fn noze_sniff_populates_reported_counts() {
    let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("m.py") else {
        return;
    };
    std::fs::write(
        &repo.file,
        "def orphan():\n    return 1\n\ndef used():\n    return 2\n\nprint(used())\n",
    )
    .unwrap();

    let resp = call_tool("noze_sniff", json!({"path": repo.path}));
    assert_eq!(resp["isError"], false);

    let report = brainz_report_for(&repo.path);
    assert_eq!(
        report["all_time"]["scans"], 1,
        "noze_sniff must record a scan"
    );
    assert_eq!(
        report["all_time"]["scans_by_origin"]["tool"], 1,
        "noze_sniff scans are tagged with the tool origin"
    );
    assert!(
        report["all_time"]["reported_by_detector"]["dead_code/function"]
            .as_u64()
            .unwrap_or(0)
            >= 1,
        "noze_sniff must populate reported_by_detector: {}",
        report["all_time"]["reported_by_detector"]
    );
}

/// `noze_gate` must also record a Scan event (the gate sees the
/// same findings the tool would) and a GateBlock event when it
/// blocks. A regression that drops either would silently zero out
/// the gate-funnel / conversion signals.
#[test]
fn noze_gate_populates_scan_and_block_metrics() {
    let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };
    std::fs::write(&repo.file, "def orphan():\n    return 1\n").unwrap();

    let resp = call_tool("noze_gate", json!({"path": repo.path}));
    assert_eq!(resp["isError"], false);
    assert_block_decision(&resp);

    let report = brainz_report_for(&repo.path);
    assert_eq!(
        report["all_time"]["scans"], 1,
        "noze_gate must record a scan"
    );
    assert_eq!(
        report["all_time"]["scans_by_origin"]["gate"], 1,
        "noze_gate scans are tagged with the gate origin"
    );
    assert!(
        report["all_time"]["reported_by_detector"]["dead_code/function"]
            .as_u64()
            .unwrap_or(0)
            >= 1,
        "noze_gate must populate reported_by_detector: {}",
        report["all_time"]["reported_by_detector"]
    );
    assert_eq!(
        report["all_time"]["gate_blocks"], 1,
        "a blocking gate call must record one block"
    );
}

/// When the gate allows because there is nothing to nag about
/// (no diff vs. head), the gate short-circuits before the scan
/// runs. The metrics counters stay at zero — fingerprinting
/// something identical to the last scan would be wasted I/O.
#[test]
fn noze_gate_skips_scan_when_nothing_changed() {
    let _metrics = crate::test_support::metrics_guard();
    let Some(repo) = fresh_repo("added.py") else {
        return;
    };
    // No edits — the gate will allow.

    let resp = call_tool("noze_gate", json!({"path": repo.path}));
    assert_eq!(resp["isError"], false);
    assert!(
        resp["content"][0]["text"].as_str() == Some("{}"),
        "clean gate call must allow: {resp:?}"
    );

    let report = brainz_report_for(&repo.path);
    assert_eq!(
        report["all_time"]["scans"], 0,
        "no scan runs when the diff is empty"
    );
    assert_eq!(report["all_time"]["gate_blocks"], 0, "no block was issued");
}

fn call_tool(name: &str, args: Value) -> Value {
    handle_message(&json!({
        "jsonrpc": "2.0", "id": 99, "method": "tools/call",
        "params": {"name": name, "arguments": args},
    }))
    .unwrap()["result"]
        .clone()
}

fn assert_block_decision(resp: &Value) {
    let text = resp["content"][0]["text"].as_str().unwrap();
    let decision: Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        decision["decision"], "block",
        "expected a block, got {text}"
    );
}

fn brainz_report_for(path: &str) -> Value {
    let resp = call_tool("brainz_report", json!({"path": path}));
    let text = resp["content"][0]["text"].as_str().unwrap();
    serde_json::from_str(text).unwrap()
}

fn fresh_repo(scratch: &str) -> Option<GitTestRepo> {
    GitTestRepo::importing(scratch, "m")
}

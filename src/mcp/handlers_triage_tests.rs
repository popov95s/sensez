use serde_json::json;

#[test]
fn triage_with_unknown_verdict_is_tool_error() {
    let args = json!({"path": ".", "pillar": "dead_code", "match": "x", "verdict": "bogus"});
    let resp = super::call("brainz_triage", &args).unwrap();
    assert_eq!(resp["isError"], true);
    let text = resp["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "verdict must be debt | false_positive | clear");
}

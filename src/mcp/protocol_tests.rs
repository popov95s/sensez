use super::*;

#[test]
fn initialize_returns_server_info() {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"});
    let resp = handle_message(&req).unwrap();
    assert_eq!(resp["result"]["serverInfo"]["name"], "sensez");
}

#[test]
fn unknown_protocol_version_gets_the_latest_supported() {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2099-01-01"}});
    let resp = handle_message(&req).unwrap();
    assert_eq!(resp["result"]["protocolVersion"], "2025-06-18");
}

#[test]
fn tool_call_without_a_name_says_so() {
    let req = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {}});
    let resp = handle_message(&req).unwrap();
    assert_eq!(resp["error"]["message"], "missing 'name'");
}

#[test]
fn notification_yields_no_response() {
    let note = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    assert!(handle_message(&note).is_none());
}

#[test]
fn unknown_method_is_error() {
    let req = json!({"jsonrpc": "2.0", "id": 2, "method": "bogus"});
    let resp = handle_message(&req).unwrap();
    assert_eq!(resp["error"]["code"], -32601);
}

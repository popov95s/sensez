//! JSON-RPC message dispatch for the MCP stdio server.

use super::handlers::{self, ToolResult};
use serde_json::{json, Value};

/// Protocol versions this server can speak, oldest first. A supported request is
/// echoed back; anything else gets the latest, as the MCP spec asks.
const SUPPORTED_PROTOCOL_VERSIONS: [&str; 3] = ["2024-11-05", "2025-03-26", "2025-06-18"];
const LATEST_PROTOCOL_VERSION: &str =
    SUPPORTED_PROTOCOL_VERSIONS[SUPPORTED_PROTOCOL_VERSIONS.len() - 1];

pub fn handle_message(msg: &Value) -> Option<Value> {
    let Some(method) = msg.get("method") else {
        let id = msg.get("id").cloned();
        return id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32600, "message": "invalid request: missing method"}
            })
        });
    };
    let outcome = match method.as_str() {
        Some("initialize") => Ok(initialize_result(
            msg.get("params")
                .and_then(|params| params.get("protocolVersion")),
        )),
        Some("tools/list") => Ok(super::tools::tools_list()),
        Some("prompts/list") => Ok(super::prompts::prompts_list()),
        Some("prompts/get") => super::prompts::prompts_get(msg.get("params")),
        Some("tools/call") => handle_tool_call(msg.get("params")),
        Some("ping") => Ok(json!({})),
        Some(other) => Err((-32601, format!("method not found: {other}"))),
        // Non-string method values are still answered with -32601; format the
        // raw JSON so the message is never an empty interpolation.
        None => Err((-32601, format!("method not found: {method}"))),
    };

    let id = msg.get("id").cloned()?;
    Some(match outcome {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
        }
    })
}

fn initialize_result(requested: Option<&Value>) -> Value {
    let negotiated = requested
        .and_then(Value::as_str)
        .filter(|version| SUPPORTED_PROTOCOL_VERSIONS.contains(version))
        .unwrap_or(LATEST_PROTOCOL_VERSION);
    json!({
        "protocolVersion": negotiated,
        "capabilities": {"tools": {}, "prompts": {}},
        "serverInfo": {"name": "sensez", "version": env!("CARGO_PKG_VERSION")}
    })
}

fn handle_tool_call(params: Option<&Value>) -> ToolResult {
    let request = params.ok_or_else(|| (handlers::INVALID_PARAMS, "missing params".to_string()))?;
    let name = request
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let args = request
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    handlers::call(name, &args)
}

#[cfg(test)]
mod tests {
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
}

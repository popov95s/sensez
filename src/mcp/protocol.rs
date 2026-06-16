//! JSON-RPC message dispatch for the MCP stdio server.

use super::handlers::{self, ToolResult};
use serde_json::{json, Value};

pub fn handle_message(msg: &Value) -> Option<Value> {
    let method = msg
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let outcome = match method {
        "initialize" => Ok(initialize_result()),
        "tools/list" => Ok(super::tools::tools_list()),
        "prompts/list" => Ok(super::prompts::prompts_list()),
        "prompts/get" => super::prompts::prompts_get(msg.get("params")),
        "tools/call" => handle_tool_call(msg.get("params")),
        "ping" => Ok(json!({})),
        other => Err((-32601, format!("method not found: {other}"))),
    };

    let id = msg.get("id").cloned()?;
    Some(match outcome {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, message)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
        }
    })
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {"tools": {}, "prompts": {}},
        "serverInfo": {"name": "sensez", "version": env!("CARGO_PKG_VERSION")}
    })
}

fn handle_tool_call(params: Option<&Value>) -> ToolResult {
    let params = params.ok_or((-32602, "missing params".to_string()))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let args = params
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

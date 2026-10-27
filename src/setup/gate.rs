//! Claude Code gate hooks that pass existing transcript metadata to Sensez.

use super::providers::{load_json_config, write_json};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

/// How long the MCP server works on one request; the gate hook waits as long.
pub(crate) const MCP_REQUEST_TIMEOUT_SECS: u64 = 600;

pub(super) fn write(root: &Path) -> Result<String> {
    let path = root.join(".claude/settings.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut settings = load_json_config(&path)?.unwrap_or_else(|| json!({}));
    install(&mut settings, "Stop", gate_hook())?;
    remove_legacy_subagent_hook(&mut settings);
    write_json(&path, &settings)?;
    Ok("installed session-scoped Claude Code gate hook".into())
}

fn install(settings: &mut Value, event: &str, hook: Value) -> Result<()> {
    let Some(map) = settings.as_object_mut() else {
        anyhow::bail!("settings must contain a JSON object");
    };
    let hooks = map.entry("hooks".to_string()).or_insert_with(|| json!({}));
    let Some(hooks) = hooks.as_object_mut() else {
        anyhow::bail!("`hooks` must be a JSON object");
    };
    let slot = hooks.entry(event.to_string()).or_insert_with(|| json!([]));
    let Some(event_hooks) = slot.as_array_mut() else {
        anyhow::bail!("`hooks.{event}` must be an array");
    };
    if let Some(existing) = event_hooks.iter_mut().find(|value| is_gate_hook(value)) {
        *existing = hook;
    } else {
        event_hooks.push(hook);
    }
    Ok(())
}

/// A hook group that runs the sensez gate, in the current or any earlier install format.
fn is_gate_hook(value: &Value) -> bool {
    value
        .get("hooks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(runs_gate)
}

fn runs_gate(hook: &Value) -> bool {
    let field = |key| hook.get(key).and_then(Value::as_str).unwrap_or_default();
    let legacy_tool = field("tool") == "gate" && matches!(field("server"), "sensez" | "sense");
    let gate_command = field("command").contains("sensez") && field("command").contains("gate");
    field("tool") == "noze_gate" || legacy_tool || gate_command
}

fn remove_legacy_subagent_hook(settings: &mut Value) {
    let Some(hooks) = settings
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .and_then(|map| map.get_mut("SubagentStop"))
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    hooks.retain(|value| !is_gate_hook(value));
}

fn gate_hook() -> Value {
    let input = json!({
        "path": "${cwd}",
        "session_id": "${session_id}",
        "transcript_path": "${transcript_path}",
        "stop_hook_active": "${stop_hook_active}"
    });
    json!({"hooks": [{
        "type": "mcp_tool", "server": "sensez", "tool": "noze_gate", "input": input,
        "timeout": MCP_REQUEST_TIMEOUT_SECS,
        "statusMessage": "sensez: experimental stop hook scans this session's transcript"
    }]})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earlier_gate_hook_formats_are_recognised() {
        for hook in [
            json!({"type": "mcp_tool", "tool": "noze_gate"}),
            json!({"type": "mcp_tool", "server": "sense", "tool": "gate"}),
            json!({"type": "command", "command": "npx sensez mcp gate"}),
        ] {
            assert!(is_gate_hook(&json!({"hooks": [hook]})), "{hook}");
        }
    }

    #[test]
    fn unrelated_hooks_are_kept() {
        for hook in [
            json!({"type": "mcp_tool", "server": "other", "tool": "gate"}),
            json!({"type": "command", "command": "keep-me"}),
        ] {
            assert!(!is_gate_hook(&json!({"hooks": [hook]})), "{hook}");
        }
    }

    #[test]
    fn reinstall_replaces_a_legacy_gate_instead_of_adding_one() {
        let mut settings = json!({"hooks": {"Stop": [
            {"hooks": [{"type": "mcp_tool", "server": "sense", "tool": "gate"}]}
        ]}});
        install(&mut settings, "Stop", gate_hook()).unwrap();
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 1);
        assert!(stop[0].to_string().contains("noze_gate"));
    }
}

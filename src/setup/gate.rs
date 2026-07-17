//! Claude Code gate hooks that pass existing transcript metadata to Sensez.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn write(root: &Path) -> Result<String> {
    let path = root.join(".claude/settings.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut settings: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| json!({}));
    install(&mut settings, "Stop", gate_hook());
    remove_legacy_subagent_hook(&mut settings);
    std::fs::write(&path, serde_json::to_string_pretty(&settings)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok("installed session-scoped Claude Code gate hook".into())
}

fn install(settings: &mut Value, event: &str, hook: Value) {
    if !settings["hooks"][event].is_array() {
        settings["hooks"][event] = json!([]);
    }
    let Some(hooks) = settings["hooks"][event].as_array_mut() else {
        return;
    };
    if let Some(existing) = hooks.iter_mut().find(|value| is_gate_hook(value)) {
        *existing = hook;
    } else {
        hooks.push(hook);
    }
}

fn is_gate_hook(value: &Value) -> bool {
    value.to_string().contains("\"tool\":\"noze_gate\"")
}

fn remove_legacy_subagent_hook(settings: &mut Value) {
    let Some(hooks) = settings["hooks"]["SubagentStop"].as_array_mut() else {
        return;
    };
    hooks.retain(|value| !is_gate_hook(value));
}

fn gate_hook() -> Value {
    let input = json!({
        "path": "${cwd}",
        "stop_hook_active": "${stop_hook_active}",
        "session_id": "${session_id}",
        "transcript_path": "${transcript_path}"
    });
    json!({"hooks": [{
        "type": "mcp_tool", "server": "sensez", "tool": "noze_gate", "input": input,
        "timeout": 60,
        "statusMessage": "sensez: experimental stop hook scans this session's transcript"
    }]})
}

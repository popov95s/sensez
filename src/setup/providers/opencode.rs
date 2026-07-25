use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub fn write(path: &Path, sensez_bin: &str) -> Result<()> {
    let mut config: Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| {
            json!({
                "$schema": "https://opencode.ai/config.json",
                "mcp": {}
            })
        });
    config["mcp"]["sensez"] = json!({
        "type": "local",
        "command": [sensez_bin, "mcp", "serve"],
        "enabled": true
    });
    std::fs::write(path, serde_json::to_string_pretty(&config)?)
        .with_context(|| format!("writing {}", path.display()))
}

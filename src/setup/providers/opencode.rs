//! OpenCode adapter (`opencode.jsonc`).

use super::{ensure_object, load_json_config, write_json};
use anyhow::Result;
use serde_json::json;
use std::path::Path;

pub fn write(path: &Path, sensez_bin: &str) -> Result<()> {
    let mut config = load_json_config(path)?.unwrap_or_else(|| {
        json!({
            "$schema": "https://opencode.ai/config.json",
            "mcp": {}
        })
    });
    let mcp = ensure_object(&mut config, "mcp", path)?;
    mcp["sensez"] = json!({
        "type": "local",
        "command": [sensez_bin, "mcp", "serve"],
        "enabled": true
    });
    write_json(path, &config)
}

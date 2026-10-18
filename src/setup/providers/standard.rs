//! `.mcp.json` adapter (the cross-agent convention).

use super::{ensure_object, load_json_config, write_json};
use anyhow::Result;
use serde_json::json;
use std::path::Path;

pub fn write(path: &Path, sensez_bin: &str) -> Result<()> {
    let mut config = load_json_config(path)?.unwrap_or_else(|| json!({}));
    let servers = ensure_object(&mut config, "mcpServers", path)?;
    servers["sensez"] = json!({"command": sensez_bin, "args": ["mcp", "serve"]});
    write_json(path, &config)
}

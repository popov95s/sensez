//! OpenCode adapter (`opencode.jsonc`).
//!
//! The file is JSON-with-comments by convention, which strict JSON parsing
//! rejects for any real user config. Parsing an existing file is therefore a
//! hard error (see [`super::load_json_config`]) rather than a silent reset —
//! only fresh installs create the file here.

use super::{ensure_object, load_json_config};
use anyhow::{Context, Result};
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
    std::fs::write(path, serde_json::to_string_pretty(&config)?)
        .with_context(|| format!("writing {}", path.display()))
}

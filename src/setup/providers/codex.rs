//! Codex adapter (`.codex/config.toml`).
//!
//! Edits through `toml_edit` so the user's comments and formatting survive;
//! an existing but unparseable file is a hard error, never a reset.

use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::{value, Array, DocumentMut, Item, Table};

pub fn write(path: &Path, sensez_bin: &str) -> Result<()> {
    let mut doc = match std::fs::read_to_string(path) {
        Ok(text) => text.parse::<DocumentMut>().with_context(|| {
            format!(
                "parsing {} — refusing to modify an unparseable config",
                path.display()
            )
        })?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => DocumentMut::new(),
        Err(err) => {
            return Err(anyhow::Error::new(err).context(format!("reading {}", path.display())));
        }
    };

    if doc.get("mcp_servers").is_none() {
        doc.insert("mcp_servers", Item::Table(Table::new()));
    }
    let Some(servers) = doc["mcp_servers"].as_table_mut() else {
        anyhow::bail!("{}: `mcp_servers` must be a TOML table", path.display());
    };

    let mut sensez = Table::new();
    sensez["command"] = value(sensez_bin);
    let mut args = Array::new();
    args.push("mcp");
    args.push("serve");
    sensez["args"] = value(args);
    servers.insert("sensez", Item::Table(sensez));

    std::fs::write(path, doc.to_string()).with_context(|| format!("writing {}", path.display()))
}

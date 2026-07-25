use anyhow::{Context, Result};
use std::path::Path;

pub fn write(path: &Path, sensez_bin: &str) -> Result<()> {
    let mut config: toml::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()));
    let table = config
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("{} must be a TOML table", path.display()))?;
    let mcp = table
        .entry("mcp_servers")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("mcp_servers must be a TOML table"))?;
    let mut sensez = toml::map::Map::new();
    sensez.insert(
        "command".to_string(),
        toml::Value::String(sensez_bin.to_string()),
    );
    sensez.insert(
        "args".to_string(),
        toml::Value::Array(vec![
            toml::Value::String("mcp".to_string()),
            toml::Value::String("serve".to_string()),
        ]),
    );
    mcp.insert("sensez".to_string(), toml::Value::Table(sensez));
    std::fs::write(path, toml::to_string_pretty(&config)?)
        .with_context(|| format!("writing {}", path.display()))
}

mod codex;
mod opencode;
mod standard;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpAdapter {
    Standard,
    Codex,
    OpenCode,
}

impl McpAdapter {
    pub fn write(self, path: &Path, sensez_bin: &str) -> Result<()> {
        match self {
            Self::Standard => standard::write(path, sensez_bin),
            Self::Codex => codex::write(path, sensez_bin),
            Self::OpenCode => opencode::write(path, sensez_bin),
        }
    }
}

/// Load an existing JSON config for in-place editing.
pub(crate) fn load_json_config(path: &Path) -> Result<Option<Value>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(anyhow::Error::new(err).context(format!("reading {}", path.display())));
        }
    };
    serde_json::from_str(&text).map(Some).with_context(|| {
        format!(
            "parsing {} — refusing to modify an unparseable config \
                 (JSONC comments are not supported; register sensez manually)",
            path.display()
        )
    })
}

/// Mutable child object at `key`, creating it when missing/null and failing
/// with a typed error when the key holds any other JSON type.
pub(crate) fn ensure_object<'a>(
    parent: &'a mut Value,
    key: &str,
    display_path: &Path,
) -> Result<&'a mut Value> {
    let Some(map) = parent.as_object_mut() else {
        anyhow::bail!("{} must contain a JSON object", display_path.display());
    };
    let slot = map.entry(key.to_string()).or_insert_with(|| json!({}));
    if !slot.is_object() {
        anyhow::bail!(
            "{}: `{key}` must be a JSON object, found {}",
            display_path.display(),
            json_type_name(slot)
        );
    }
    Ok(slot)
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_uses_adapter_not_filename() {
        let tmp = tempfile::tempdir().unwrap();
        let misleading = tmp.path().join("opencode.json");
        McpAdapter::Standard
            .write(&misleading, "/bin/sensez")
            .unwrap();
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(misleading).unwrap()).unwrap();
        assert!(config["mcpServers"]["sensez"].is_object());
        assert!(config["mcp"].is_null());
    }

    #[test]
    fn unparseable_existing_config_is_an_error_not_a_reset() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".mcp.json");
        std::fs::write(&path, "{ not json }").unwrap();

        let err = McpAdapter::Standard
            .write(&path, "/bin/sensez")
            .unwrap_err();
        assert!(err.to_string().contains("refusing to modify"), "{err:#}");

        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, "{ not json }", "original file must be untouched");
    }

    /// OpenCode's config is `.jsonc`; comments make strict JSON parsing fail,
    /// which previously reset the whole file to just the sensez entry.
    #[test]
    fn jsonc_commented_config_is_refused_not_wiped() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("opencode.jsonc");
        std::fs::write(&path, "// my config\n{\"mcp\": {}}\n").unwrap();

        assert!(McpAdapter::OpenCode.write(&path, "/bin/sensez").is_err());
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.contains("// my config"), "comments must survive");
    }

    /// `"mcpServers": []` used to panic the process via serde_json IndexMut.
    #[test]
    fn wrong_typed_keys_error_instead_of_panic() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".mcp.json");
        std::fs::write(&path, r#"{"mcpServers": []}"#).unwrap();

        let err = McpAdapter::Standard
            .write(&path, "/bin/sensez")
            .unwrap_err();
        assert!(err.to_string().contains("must be a JSON object"), "{err:#}");
    }
}

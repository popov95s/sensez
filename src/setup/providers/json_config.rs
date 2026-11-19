use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

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

/// Write `value` as pretty JSON with a trailing newline.
pub(crate) fn write_json(path: &Path, value: &Value) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

/// Mutable child object at `key`, created when missing or null.
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

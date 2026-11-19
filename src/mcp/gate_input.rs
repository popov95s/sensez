//! Validate the Stop-hook arguments Claude Code substitutes into `noze_gate`.

use serde_json::Value;
use std::path::PathBuf;

/// The `path` argument as an existing directory, or a message saying what is wrong with it.
pub(super) fn repository_path(args: &Value) -> Result<PathBuf, String> {
    let Some(raw) = args.get("path").and_then(Value::as_str) else {
        return Err("sensez gate: missing 'path' argument; pass the repository root".into());
    };
    if is_unsubstituted(raw) {
        return Err(format!(
            "sensez gate: 'path' is the literal template {raw}; the hook host did not \
             substitute it. Re-run `sensez init` or set the hook input path to the repository root."
        ));
    }
    let path = PathBuf::from(raw);
    if !path.is_dir() {
        return Err(format!(
            "sensez gate: 'path' {raw} is not an existing directory; set the hook input \
             path to the repository root."
        ));
    }
    Ok(path)
}

/// A string argument, unless it is empty or an unsubstituted `${…}` template.
pub(super) fn templated<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && !is_unsubstituted(value))
}

/// Whether Claude Code says this Stop already follows a Stop-hook block.
pub(super) fn stop_hook_active(args: &Value) -> bool {
    match args.get("stop_hook_active") {
        Some(Value::Bool(active)) => *active,
        Some(Value::String(text)) => text == "true",
        _ => false,
    }
}

fn is_unsubstituted(value: &str) -> bool {
    value.contains("${")
}

#[cfg(test)]
#[path = "gate_input_tests.rs"]
mod tests;

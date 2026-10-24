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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unsubstituted_path_is_an_error_naming_the_template() {
        let err = repository_path(&json!({"path": "${cwd}"})).unwrap_err();
        assert!(err.contains("${cwd}"), "{err}");
    }

    #[test]
    fn missing_directory_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let gone = tmp.path().join("gone");
        let err = repository_path(&json!({"path": gone})).unwrap_err();
        assert!(err.contains("not an existing directory"), "{err}");
    }

    #[test]
    fn unsubstituted_optional_fields_count_as_absent() {
        let args = json!({"session_id": "${session_id}", "transcript_path": ""});
        assert_eq!(templated(&args, "session_id"), None);
        assert_eq!(templated(&args, "transcript_path"), None);
    }

    #[test]
    fn stop_hook_active_accepts_bool_and_substituted_string() {
        assert!(stop_hook_active(&json!({"stop_hook_active": true})));
        assert!(stop_hook_active(&json!({"stop_hook_active": "true"})));
        assert!(!stop_hook_active(
            &json!({"stop_hook_active": "${stop_hook_active}"})
        ));
        assert!(!stop_hook_active(&json!({})));
    }
}

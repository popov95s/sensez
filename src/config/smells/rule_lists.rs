//! List-valued rule knobs.

use crate::report::SmellKind;

pub(super) fn globs(kind: SmellKind, key: &str, items: Vec<String>) -> Result<Vec<String>, String> {
    crate::globs::build_globset(&items)
        .map_err(|err| format!("[smells.rules.{kind}] {key}: {err}"))?;
    Ok(items)
}

pub(super) fn string_list(value: &toml::Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect()
}

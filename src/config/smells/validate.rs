use crate::config::smells::knobs::Smells;
use std::collections::BTreeSet;

/// Reject any keys in `table` that are not valid `Smells` field names.
pub(super) fn validate_keys(scope: &str, table: &toml::Table) -> Result<(), String> {
    let allowed = allowed_keys();
    let unknown: Vec<_> = table
        .keys()
        .filter(|key| !allowed.contains(*key))
        .cloned()
        .collect();
    if unknown.is_empty() {
        Ok(())
    } else {
        let label = if scope == "<base>" {
            "[smells]".to_string()
        } else {
            format!("[smells.{scope}]")
        };
        Err(format!("unknown {label} key(s): {}", unknown.join(", ")))
    }
}

/// Cache of the set of `Smells` field names (derived from
/// `Smells::default()`). Rebuilt once per call — the per-language tables
/// are small (a few entries) so the cost is negligible.
fn allowed_keys() -> BTreeSet<String> {
    match toml::Value::try_from(Smells::default()) {
        Ok(toml::Value::Table(table)) => table.keys().cloned().collect(),
        _ => BTreeSet::new(),
    }
}

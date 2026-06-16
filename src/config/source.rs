//! Config discovery: `sensez.toml` first; else a `[tool.sensez]` table in
//! `pyproject.toml` for projects that centralize tool config there.

use anyhow::{Context, Result};
use std::path::Path;

pub(super) enum Source {
    /// Raw `sensez.toml` text.
    SensezToml(String),
    /// The extracted `[tool.sensez]` table from `pyproject.toml`.
    Pyproject(toml::Value),
    /// Neither present — run on defaults.
    Defaults,
}

/// Locate the repo's sensez configuration. `sensez.toml` always wins; a broken
/// `pyproject.toml` is ignored here, but a present `[tool.sensez]` table that
/// fails to deserialize errors later so typos surface rather than silently
/// reverting to defaults.
pub(super) fn discover(project_root: &Path) -> Result<Source> {
    let sensez = project_root.join("sensez.toml");
    if sensez.exists() {
        let text = std::fs::read_to_string(&sensez)
            .with_context(|| format!("reading {}", sensez.display()))?;
        return Ok(Source::SensezToml(text));
    }
    let pyproject = project_root.join("pyproject.toml");
    if let Ok(text) = std::fs::read_to_string(&pyproject) {
        if let Ok(value) = text.parse::<toml::Value>() {
            if let Some(table) = value.get("tool").and_then(|t| t.get("sensez")) {
                return Ok(Source::Pyproject(table.clone()));
            }
        }
    }
    Ok(Source::Defaults)
}

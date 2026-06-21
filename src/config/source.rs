//! Config discovery: `sensez.toml` first; else a `[tool.sensez]` table in
//! `pyproject.toml` for projects that centralize tool config there.

use crate::report::{ScanIssue, ScanStage};
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

pub(super) fn discover_with_issues(project_root: &Path) -> (Source, Vec<ScanIssue>) {
    let sensez = project_root.join("sensez.toml");
    if sensez.exists() {
        return match std::fs::read_to_string(&sensez) {
            Ok(text) => (Source::SensezToml(text), Vec::new()),
            Err(err) => (
                Source::Defaults,
                vec![issue(Some(sensez), format!("reading sensez.toml: {err}"))],
            ),
        };
    }

    let pyproject = project_root.join("pyproject.toml");
    match std::fs::read_to_string(&pyproject) {
        Ok(text) => match text.parse::<toml::Value>() {
            Ok(value) => match value.get("tool").and_then(|t| t.get("sensez")) {
                Some(table) => (Source::Pyproject(table.clone()), Vec::new()),
                None => (Source::Defaults, Vec::new()),
            },
            Err(err) => (
                Source::Defaults,
                vec![issue(
                    Some(pyproject),
                    format!("parsing pyproject.toml: {err}"),
                )],
            ),
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (Source::Defaults, Vec::new()),
        Err(err) => (
            Source::Defaults,
            vec![issue(
                Some(pyproject),
                format!("reading pyproject.toml: {err}"),
            )],
        ),
    }
}

fn issue(file: Option<std::path::PathBuf>, message: String) -> ScanIssue {
    ScanIssue {
        stage: ScanStage::Config,
        file,
        message,
    }
}

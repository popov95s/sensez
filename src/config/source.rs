//! Config discovery: `sensez.toml` first; else a `[tool.sensez]` table in
//! `pyproject.toml` for projects that centralize tool config there.

use super::problem::ConfigProblem;
use std::path::Path;

pub(super) enum Source {
    /// Raw `sensez.toml` text.
    SensezToml(String),
    /// Raw `pyproject.toml` text; its `[tool.sensez]` table, if any, is the config.
    Pyproject(String),
    /// Neither present — run on defaults.
    Defaults,
}

impl Source {
    pub(super) fn file(&self) -> &'static str {
        match self {
            Source::SensezToml(_) | Source::Defaults => "sensez.toml",
            Source::Pyproject(_) => "pyproject.toml",
        }
    }
}

/// Locate the repo's sensez configuration. `sensez.toml` always wins.
pub(super) fn discover(project_root: &Path) -> Result<Source, ConfigProblem> {
    let sensez = project_root.join("sensez.toml");
    if sensez.exists() {
        return read(&sensez, "sensez.toml").map(Source::SensezToml);
    }
    let pyproject = project_root.join("pyproject.toml");
    if pyproject.exists() {
        return read(&pyproject, "pyproject.toml").map(Source::Pyproject);
    }
    Ok(Source::Defaults)
}

fn read(path: &Path, file: &'static str) -> Result<String, ConfigProblem> {
    std::fs::read_to_string(path).map_err(|err| ConfigProblem {
        file,
        position: None,
        message: format!("reading {}: {err}", path.display()),
    })
}

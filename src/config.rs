//! `sensez.toml` facade: public config API plus load/normalization plumbing.

mod brainz;
pub mod model;
pub mod smells;
mod source;

pub use brainz::SelfImprovement;
pub use model::{ActionPolicy, Boundaries, Config, DeadCode, Duplication, ForbiddenRule, Gate};
pub use smells::{SmellConfig, Smells};

use crate::report::{ScanIssue, ScanStage};
use anyhow::{Context, Result};
use std::path::Path;

/// Vendored/generated/build artifacts are not owned source. Exclude them from
/// discovery for every pillar so minified bundles and checked-in third-party
/// code never dominate repo reports.
pub(crate) const GLOBAL_BASELINE_EXCLUDE: [&str; 12] = [
    "**/node_modules/**",
    "**/.venv/**",
    "**/venv/**",
    "**/target/**",
    "**/dist/**",
    "**/build/**",
    "**/vendor/**",
    "**/_vendor/**",
    "**/vendors/**",
    "**/generated/**",
    "**/*.min.js",
    "**/*.min.css",
];

/// Tests and migration scripts are reached outside normal product flow, so their
/// boilerplate isn't actionable duplication/smell noise. Language-specific
/// dead-code entry points live on the profiles instead, where pytest/alembic
/// conventions cannot leak into JS/TS/Rust.
pub(crate) const BASELINE_EXCLUDE: [&str; 8] = [
    "**/tests/**",
    "**/test/**",
    "**/conftest.py",
    "**/test_*.py",
    "**/*_test.py",
    "**/alembic/**",
    "**/migrations/**",
    "**/versions/**",
];

impl Config {
    /// Load configuration: `sensez.toml`, else `[tool.sensez]` from
    /// `pyproject.toml`, else defaults (see [`source`]).
    pub fn load(project_root: &Path) -> Result<Config> {
        let mut config: Config = match source::discover(project_root)? {
            source::Source::SensezToml(text) => {
                toml::from_str(&text).context("parsing sensez.toml")?
            }
            source::Source::Pyproject(table) => table
                .try_into()
                .context("parsing [tool.sensez] in pyproject.toml")?,
            source::Source::Defaults => Config::default(),
        };
        // Resolve configured roots to absolute paths under the project root.
        config.roots = config.roots.iter().map(|r| project_root.join(r)).collect();
        config.apply_baseline_excludes();
        config.validate_globs()?;
        Ok(config)
    }

    /// Load configuration for a scan. Invalid project-local config degrades to
    /// defaults and emits structured warnings so analysis can still complete.
    pub(crate) fn load_for_scan(project_root: &Path) -> (Config, Vec<ScanIssue>) {
        let (source, mut issues) = source::discover_with_issues(project_root);
        let issue_file = config_issue_file(&source, project_root);
        let mut config = match parse_source(source) {
            Ok(config) => config,
            Err(err) => {
                issues.push(config_issue(issue_file.clone(), format!("{err:#}")));
                Config::default()
            }
        };
        config.roots = config.roots.iter().map(|r| project_root.join(r)).collect();
        config.apply_baseline_excludes();
        if let Err(err) = config.validate_globs() {
            issues.push(config_issue(issue_file, format!("{err:#}")));
            config = Config::default();
            config.roots = config.roots.iter().map(|r| project_root.join(r)).collect();
            config.apply_baseline_excludes();
        }
        (config, issues)
    }

    /// Merge the built-in test/migration globs into both pillars (idempotent).
    fn apply_baseline_excludes(&mut self) {
        for glob in GLOBAL_BASELINE_EXCLUDE {
            let g = glob.to_string();
            if !self.exclude.contains(&g) {
                self.exclude.push(g);
            }
        }
        for glob in BASELINE_EXCLUDE {
            let g = glob.to_string();
            if !self.duplication.exclude.contains(&g) {
                self.duplication.exclude.push(g.clone());
            }
            if !self.smells.exclude.contains(&g) {
                self.smells.exclude.push(g);
            }
        }
    }

    fn validate_globs(&self) -> Result<()> {
        crate::globs::validate_patterns("exclude", &self.exclude)?;
        crate::globs::validate_patterns("duplication.exclude", &self.duplication.exclude)?;
        crate::globs::validate_patterns("dead_code.entry_points", &self.dead_code.entry_points)?;
        crate::globs::validate_patterns("smells.exclude", &self.smells.exclude)?;
        Ok(())
    }
}

fn parse_source(source: source::Source) -> Result<Config> {
    match source {
        source::Source::SensezToml(text) => toml::from_str(&text).context("parsing sensez.toml"),
        source::Source::Pyproject(table) => table
            .try_into()
            .context("parsing [tool.sensez] in pyproject.toml"),
        source::Source::Defaults => Ok(Config::default()),
    }
}

fn config_issue_file(source: &source::Source, project_root: &Path) -> Option<std::path::PathBuf> {
    match source {
        source::Source::SensezToml(_) => Some(project_root.join("sensez.toml")),
        source::Source::Pyproject(_) => Some(project_root.join("pyproject.toml")),
        source::Source::Defaults => None,
    }
}

fn config_issue(file: Option<std::path::PathBuf>, message: String) -> ScanIssue {
    ScanIssue {
        stage: ScanStage::Config,
        file,
        message,
    }
}

#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;

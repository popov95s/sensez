//! `sensez.toml` facade: public config API plus load/normalization plumbing.

mod brainz;
pub mod model;
mod parse;
mod problem;
pub mod smells;
mod source;
mod suggest;
mod unknown_keys;

pub use brainz::SelfImprovement;
pub use model::{
    ActionPolicy, Boundaries, Cache, Config, Cycles, DeadCode, Duplication, ForbiddenRule,
    SemanticDuplication,
};
pub use problem::{ConfigErrors, ConfigProblem};
pub use smells::{SmellConfig, Smells};

use crate::report::{ScanIssue, ScanStage};
use anyhow::Result;
use std::path::Path;

/// Vendored/generated/build artifacts and documentation applications are not
/// owned product source. Exclude them from discovery for every pillar so they
/// never dominate repository findings.
pub(crate) const GLOBAL_BASELINE_EXCLUDE: [&str; 13] = [
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
    "**/docs/**",
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
    /// `pyproject.toml`, else defaults (see [`source`]). Warnings are dropped.
    pub fn load(project_root: &Path) -> Result<Config> {
        Ok(Self::load_with_warnings(project_root)?.0)
    }

    /// Strict load: `Err` lists every error; `Ok` carries non-fatal warnings.
    pub fn load_with_warnings(
        project_root: &Path,
    ) -> Result<(Config, Vec<ConfigProblem>), ConfigErrors> {
        let source =
            source::discover(project_root).map_err(|problem| ConfigErrors(vec![problem]))?;
        let (mut config, warnings) = parse::parse(&source).map_err(ConfigErrors)?;
        config.resolve_paths(project_root);
        config.validate().map_err(|err| {
            ConfigErrors(vec![ConfigProblem {
                file: source.file(),
                position: None,
                message: format!("{err:#}"),
            }])
        })?;
        Ok((config, warnings))
    }

    /// Load configuration for a scan; invalid config runs on defaults and every problem becomes a [`ScanIssue`].
    pub(crate) fn load_for_scan(project_root: &Path) -> (Config, Vec<ScanIssue>) {
        match Self::load_with_warnings(project_root) {
            Ok((config, warnings)) => (config, warnings.into_iter().map(config_issue).collect()),
            Err(errors) => {
                let mut config = Config::default();
                config.resolve_paths(project_root);
                (config, errors.0.into_iter().map(config_issue).collect())
            }
        }
    }

    fn resolve_paths(&mut self, project_root: &Path) {
        self.roots = self.roots.iter().map(|r| project_root.join(r)).collect();
        self.apply_baseline_excludes();
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
                self.smells.exclude.push(g.clone());
            }
            if !self.cycles.exclude.contains(&g) {
                self.cycles.exclude.push(g);
            }
        }
    }

    fn validate(&self) -> Result<()> {
        self.validate_globs()?;
        self.validate_semantic()
    }

    fn validate_semantic(&self) -> Result<()> {
        let semantic = &self.duplication.semantic;
        if semantic.comment_boost_score < 50 {
            anyhow::bail!(
                "duplication.semantic.comment_boost_score must be at least 50, got {}",
                semantic.comment_boost_score
            );
        }
        if semantic.min_shape_score > 100 {
            anyhow::bail!(
                "duplication.semantic.min_shape_score must be at most 100, got {}",
                semantic.min_shape_score
            );
        }
        Ok(())
    }

    fn validate_globs(&self) -> Result<()> {
        crate::globs::validate_patterns("exclude", &self.exclude)?;
        crate::globs::validate_patterns("duplication.exclude", &self.duplication.exclude)?;
        crate::globs::validate_patterns("dead_code.entry_points", &self.dead_code.entry_points)?;
        crate::globs::validate_patterns("dead_code.entrypoints", &self.dead_code.entrypoints)?;
        crate::globs::validate_patterns("cycles.exclude", &self.cycles.exclude)?;
        crate::globs::validate_patterns("smells.exclude", &self.smells.exclude)?;
        Ok(())
    }
}

fn config_issue(problem: ConfigProblem) -> ScanIssue {
    ScanIssue {
        stage: ScanStage::Config,
        file: None,
        message: problem.to_string(),
    }
}

#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "config/semantic_tests.rs"]
mod semantic_tests;

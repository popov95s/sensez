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

/// Vendored, generated, build and docs paths, excluded from discovery for every pillar.
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

/// Test layouts outside profile test globs; skipped by duplication/smells unless `include_tests`.
pub(crate) const BASELINE_TEST_EXCLUDE: [&str; 5] = [
    "**/tests/**",
    "**/test/**",
    "**/conftest.py",
    "**/test_*.py",
    "**/*_test.py",
];

/// Generated, often cyclic migration scripts; `alembic`/`versions` globs are Python-only.
pub(crate) const BASELINE_MIGRATION_EXCLUDE: [&str; 3] = [
    "**/alembic/**/*.py",
    "**/migrations/**",
    "**/versions/**/*.py",
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
        config
            .validate_globs()
            .and_then(|()| config.validate_semantic())
            .map_err(|err| {
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

    /// Merge the built-in global and migration globs into the pillar excludes (idempotent).
    fn apply_baseline_excludes(&mut self) {
        push_unique(&mut self.exclude, &GLOBAL_BASELINE_EXCLUDE);
        push_unique(&mut self.duplication.exclude, &BASELINE_MIGRATION_EXCLUDE);
        push_unique(&mut self.smells.exclude, &BASELINE_MIGRATION_EXCLUDE);
        push_unique(&mut self.cycles.exclude, &BASELINE_MIGRATION_EXCLUDE);
    }

    fn validate_semantic(&self) -> Result<()> {
        let semantic = &self.duplication.semantic;
        if !(50..=100).contains(&semantic.comment_boost_score) {
            anyhow::bail!(
                "duplication.semantic.comment_boost_score must be between 50 and 100, got {}",
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
        crate::globs::validate_patterns("dead_code.test_sources", &self.dead_code.test_sources)?;
        crate::globs::validate_patterns("dead_code.test_support", &self.dead_code.test_support)?;
        crate::globs::validate_patterns("cycles.exclude", &self.cycles.exclude)?;
        crate::globs::validate_patterns("smells.exclude", &self.smells.exclude)?;
        let boundary_patterns: Vec<String> = self
            .boundaries
            .forbidden
            .iter()
            .flat_map(|rule| [rule.from.clone(), rule.to.clone()])
            .collect();
        crate::globs::validate_patterns("boundaries.forbidden", &boundary_patterns)?;
        Ok(())
    }
}

fn push_unique(target: &mut Vec<String>, globs: &[&str]) {
    for glob in globs {
        if !target.iter().any(|existing| existing == glob) {
            target.push((*glob).to_string());
        }
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

//! Which parsed files the source-level pillars (smells, duplication) analyse.

use crate::config::model::{Config, Duplication};
use crate::config::smells::SmellConfig;
use crate::globs::compile_validated;
use crate::profiles::registry;
use crate::report::SkipCounts;
use crate::spine::parser::ParsedFile;
use globset::GlobSet;
use std::sync::LazyLock;

/// Test directories in any language; per-language test files come from the profiles.
static TEST_LAYOUTS: LazyLock<GlobSet> =
    LazyLock::new(|| compile_validated(&["**/tests/**", "**/test/**"]));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Inclusion {
    Skip,
    Include,
}

impl Inclusion {
    fn from_flag(include: bool) -> Self {
        if include {
            Inclusion::Include
        } else {
            Inclusion::Skip
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipReason {
    ConfigExclude,
    Test,
    Generated,
}

pub(crate) struct SourceScope {
    excluded: GlobSet,
    tests: Inclusion,
    generated: Inclusion,
}

/// The file scopes of the source-level pillars, compiled once per scan.
pub(crate) struct PillarScopes {
    pub(crate) duplication: SourceScope,
    pub(crate) smells: SourceScope,
}

impl PillarScopes {
    pub(crate) fn new(config: &Config) -> Self {
        Self {
            duplication: SourceScope::for_duplication(&config.duplication),
            smells: SourceScope::for_smells(&config.smells),
        }
    }
}

impl SourceScope {
    fn new(exclude: &[String], tests: Inclusion, generated: Inclusion) -> Self {
        Self {
            excluded: compile_validated(exclude),
            tests,
            generated,
        }
    }

    pub(crate) fn for_smells(cfg: &SmellConfig) -> Self {
        Self::new(
            &cfg.exclude,
            Inclusion::from_flag(cfg.include_tests),
            Inclusion::Skip,
        )
    }

    pub(crate) fn for_duplication(cfg: &Duplication) -> Self {
        Self::new(
            &cfg.exclude,
            Inclusion::from_flag(cfg.include_tests),
            Inclusion::from_flag(cfg.include_generated),
        )
    }

    pub(crate) fn skip_reason(&self, file: &ParsedFile) -> Option<SkipReason> {
        if self.excluded.is_match(&file.path) {
            return Some(SkipReason::ConfigExclude);
        }
        if self.tests == Inclusion::Skip && is_test_file(file) {
            return Some(SkipReason::Test);
        }
        if self.generated == Inclusion::Skip && file.is_generated {
            return Some(SkipReason::Generated);
        }
        None
    }

    pub(crate) fn includes(&self, file: &ParsedFile) -> bool {
        self.skip_reason(file).is_none()
    }

    pub(crate) fn skip_counts(&self, files: &[ParsedFile]) -> SkipCounts {
        let mut counts = SkipCounts::default();
        for reason in files.iter().filter_map(|file| self.skip_reason(file)) {
            match reason {
                SkipReason::ConfigExclude => counts.config_exclude += 1,
                SkipReason::Test => counts.tests += 1,
                SkipReason::Generated => counts.generated += 1,
            }
        }
        counts
    }
}

fn is_test_file(file: &ParsedFile) -> bool {
    TEST_LAYOUTS.is_match(&file.path)
        || registry::is_test_or_support_source(file.language, &file.path)
}

use globset::GlobSet;
use regex::Regex;
use std::path::{Path, PathBuf};

/// Which files one runner project collects as tests.
pub struct TestProject {
    pub dir: PathBuf,
    pub base: PathBase,
    pub roots: Vec<PathBuf>,
    pub include: Include,
    pub exclude: Exclude,
}

/// What config globs are matched against.
pub enum PathBase {
    /// Vitest: relative to the project directory.
    Relative,
    /// Jest: the absolute path.
    Absolute,
}

pub enum Include {
    Globs(GlobSet),
    Regexes(Vec<Regex>),
    NameHeuristic,
}

pub enum Exclude {
    Globs(GlobSet),
    Regexes(Vec<Regex>),
}

impl TestProject {
    pub fn is_test(&self, file: &Path) -> bool {
        if !self.roots.iter().any(|root| file.starts_with(root)) {
            return false;
        }
        let Some(subject) = self.subject(file) else {
            return false;
        };
        let included = match &self.include {
            Include::Globs(globs) => globs.is_match(&subject),
            Include::Regexes(patterns) => patterns.iter().any(|regex| regex.is_match(&subject)),
            Include::NameHeuristic => is_conventional_test(file),
        };
        let excluded = match &self.exclude {
            Exclude::Globs(globs) => globs.is_match(&subject),
            Exclude::Regexes(patterns) => patterns.iter().any(|regex| regex.is_match(&subject)),
        };
        included && !excluded
    }

    fn subject(&self, file: &Path) -> Option<String> {
        let path = match self.base {
            PathBase::Relative => file.strip_prefix(&self.dir).ok()?,
            PathBase::Absolute => file,
        };
        Some(path.to_string_lossy().replace('\\', "/"))
    }
}

/// File-name convention used when a runner's include key cannot be read.
fn is_conventional_test(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name.contains(".test.")
        || name.contains(".spec.")
        || path
            .components()
            .any(|part| part.as_os_str() == "__tests__")
}

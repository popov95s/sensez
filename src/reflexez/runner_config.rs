//! Static reader for Vitest and Jest configs: which files are tests, and which are shared setup.

mod calls;
mod defaults;
mod evaluate;
mod extglob;
mod jest;
mod key;
mod locate;
mod project;
mod projects;
mod script;
mod value;
mod vitest;

use super::model::{ConfigWarning, RunnerKind};
use crate::reflexez::patterns::normalize_path;
use key::Key;
use project::TestProject;
use std::path::{Path, PathBuf};
use value::Value;

const HEURISTIC_FALLBACK: &str = "file-name heuristic (*.test.*, *.spec.*, __tests__/)";
const SETUP_FALLBACK: &str = "setup-file changes are not detected (full run with --strict-dynamic)";

/// A runner-configured setup module; changes to it or its imports affect every test.
#[derive(Clone, Debug)]
pub struct SetupFile {
    pub path: PathBuf,
    pub config: PathBuf,
    pub key: String,
}

#[derive(Default)]
pub struct RunnerConfig {
    projects: Vec<TestProject>,
    pub setup_files: Vec<SetupFile>,
    pub warnings: Vec<ConfigWarning>,
}

impl RunnerConfig {
    pub fn load(root: &Path, runner: RunnerKind) -> Self {
        match runner {
            RunnerKind::Vitest => vitest::read(root),
            RunnerKind::Jest => jest::read(root),
            RunnerKind::Pytest => Self::default(),
        }
    }

    pub fn is_test(&self, file: &Path) -> bool {
        self.projects.iter().any(|project| project.is_test(file))
    }

    /// Project directories, used to spot nested runner roots a workspace already covers.
    pub fn project_dirs(&self) -> impl Iterator<Item = &Path> {
        self.projects.iter().map(|project| project.dir.as_path())
    }

    fn warn(&mut self, file: &Path, key: &str, reason: String, fallback: &str) {
        self.warnings.push(ConfigWarning {
            file: file.to_path_buf(),
            key: key.to_string(),
            reason,
            fallback: fallback.to_string(),
        });
    }

    /// Record `key` as setup modules resolved against `base`; bare package names are skipped.
    fn read_setup(&mut self, config: &Value, file: &Path, key: &str, base: &Path) {
        match key::read(config, key) {
            Key::Absent => {}
            Key::Unreadable(reason) => self.warn(file, key, reason, SETUP_FALLBACK),
            Key::Strings(paths) => {
                for path in paths {
                    let resolved = normalize_path(&base.join(path.replace("<rootDir>", ".")));
                    if resolved.is_file() {
                        self.setup_files.push(SetupFile {
                            path: resolved,
                            config: file.to_path_buf(),
                            key: key.to_string(),
                        });
                    }
                }
            }
        }
    }

    /// A directory-valued key such as `root` or `rootDir`, defaulting to `base`.
    fn read_dir(&mut self, config: &Value, file: &Path, key: &str, base: &Path) -> PathBuf {
        match key::read(config, key) {
            Key::Strings(paths) if paths.len() == 1 => normalize_path(&base.join(&paths[0])),
            Key::Unreadable(reason) => {
                self.warn(file, key, reason, "config directory");
                base.to_path_buf()
            }
            _ => base.to_path_buf(),
        }
    }
}

/// Compile runner globs into a set, translating picomatch extglobs.
fn glob_set(patterns: &[String]) -> Result<globset::GlobSet, String> {
    let mut builder = globset::GlobSet::builder();
    for pattern in patterns {
        let translated = extglob::to_globset(pattern)?;
        let glob = globset::GlobBuilder::new(&translated)
            .literal_separator(true)
            .empty_alternates(true)
            .build()
            .map_err(|error| error.to_string())?;
        builder.add(glob);
    }
    builder.build().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;

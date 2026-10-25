//! Pytest collection rules from the nearest `pytest.ini`, `pyproject.toml`, `tox.ini` or `setup.cfg`.

use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const DEFAULT_PYTHON_FILES: [&str; 2] = ["test_*.py", "*_test.py"];

/// One pytest rootdir: where it runs, which directories and file names it collects.
pub struct PytestConfig {
    pub dir: PathBuf,
    testpaths: Vec<PathBuf>,
    python_files: GlobSet,
}

impl PytestConfig {
    fn new(dir: &Path, testpaths: Vec<String>, python_files: Option<Vec<String>>) -> Self {
        let patterns =
            python_files.unwrap_or_else(|| DEFAULT_PYTHON_FILES.map(str::to_string).to_vec());
        let mut builder = GlobSetBuilder::new();
        for glob in patterns
            .iter()
            .filter_map(|pattern| Glob::new(pattern).ok())
        {
            builder.add(glob);
        }
        Self {
            dir: dir.to_path_buf(),
            testpaths: testpaths.iter().map(|path| dir.join(path)).collect(),
            python_files: builder.build().unwrap_or_else(|_| GlobSet::empty()),
        }
    }

    /// Whether pytest run from this rootdir collects `path`.
    pub fn collects(&self, path: &Path) -> bool {
        let named = path.extension().is_some_and(|ext| ext == "py")
            && path
                .file_name()
                .is_some_and(|name| self.python_files.is_match(name));
        named
            && (self.testpaths.is_empty()
                || self.testpaths.iter().any(|root| path.starts_with(root)))
    }
}

/// Nearest pytest configuration for each directory under one repository.
pub struct PytestConfigs {
    repository: PathBuf,
    default: PytestConfig,
    by_dir: HashMap<PathBuf, Option<PytestConfig>>,
}

impl PytestConfigs {
    pub fn new(repository: &Path) -> Self {
        Self {
            repository: repository.to_path_buf(),
            default: PytestConfig::new(repository, Vec::new(), None),
            by_dir: HashMap::new(),
        }
    }

    /// Configuration pytest applies to `file`; the defaults at the repository root without one.
    pub fn for_file(&mut self, file: &Path) -> &PytestConfig {
        let mut dir = file.parent();
        while let Some(current) = dir.filter(|current| current.starts_with(&self.repository)) {
            let found = self
                .by_dir
                .entry(current.to_path_buf())
                .or_insert_with(|| read_config(current))
                .is_some();
            if found {
                return self.by_dir[current].as_ref().unwrap_or(&self.default);
            }
            dir = current.parent();
        }
        &self.default
    }
}

fn read_config(dir: &Path) -> Option<PytestConfig> {
    if let Some(values) = ini_section(&dir.join("pytest.ini"), "[pytest]") {
        return Some(from_ini(dir, &values));
    }
    if let Some(config) = pyproject_config(dir) {
        return Some(config);
    }
    ini_section(&dir.join("tox.ini"), "[pytest]")
        .or_else(|| ini_section(&dir.join("setup.cfg"), "[tool:pytest]"))
        .map(|values| from_ini(dir, &values))
}

fn pyproject_config(dir: &Path) -> Option<PytestConfig> {
    let source = std::fs::read_to_string(dir.join("pyproject.toml")).ok()?;
    let value: toml::Value = toml::from_str(&source).ok()?;
    let options = value.get("tool")?.get("pytest")?.get("ini_options")?;
    let list = |key: &str| options.get(key).map(toml_strings);
    Some(PytestConfig::new(
        dir,
        list("testpaths").unwrap_or_default(),
        list("python_files"),
    ))
}

fn toml_strings(value: &toml::Value) -> Vec<String> {
    match value {
        toml::Value::String(text) => split_words(text),
        toml::Value::Array(items) => items
            .iter()
            .filter_map(toml::Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn from_ini(dir: &Path, values: &HashMap<String, Vec<String>>) -> PytestConfig {
    PytestConfig::new(
        dir,
        values.get("testpaths").cloned().unwrap_or_default(),
        values.get("python_files").cloned(),
    )
}

/// Keys of `section` with their whitespace-separated values, continuation lines included.
fn ini_section(path: &Path, section: &str) -> Option<HashMap<String, Vec<String>>> {
    let source = std::fs::read_to_string(path).ok()?;
    let mut lines = source.lines().skip_while(|line| line.trim() != section);
    lines.next()?;
    let mut values: HashMap<String, Vec<String>> = HashMap::new();
    let mut current: Option<String> = None;
    for line in lines.take_while(|line| !line.trim().starts_with('[')) {
        if line.starts_with(char::is_whitespace) {
            if let Some(key) = &current {
                values
                    .entry(key.clone())
                    .or_default()
                    .extend(split_words(line));
            }
        } else if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            values.insert(key.clone(), split_words(value));
            current = Some(key);
        }
    }
    Some(values)
}

fn split_words(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_string).collect()
}

#[cfg(test)]
#[path = "pytest_tests.rs"]
mod tests;

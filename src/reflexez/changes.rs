use crate::diff::git::DiffSource;
use crate::profiles::registry;
use crate::spine::ir::Language;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct ChangeScope {
    pub repository: PathBuf,
    pub files: Vec<PathBuf>,
    pub fallback_reasons: Vec<String>,
}

pub fn resolve(
    root: &Path,
    base: Option<&str>,
    source: DiffSource,
    explicit: &[PathBuf],
) -> Result<ChangeScope> {
    if !explicit.is_empty() {
        let repository = crate::diff::git::repository_root(root)?;
        let cwd = std::env::current_dir().unwrap_or_else(|_| repository.clone());
        let files: Vec<_> = explicit
            .iter()
            .map(|path| explicit_file(path, &cwd, &repository))
            .collect();
        return Ok(ChangeScope {
            fallback_reasons: fallback_reasons(&files, &[]),
            repository,
            files,
        });
    }
    let changed = crate::diff::git::changed_paths(root, base, source)?;
    Ok(ChangeScope {
        repository: changed.root,
        fallback_reasons: fallback_reasons(&changed.paths, &changed.deleted),
        files: changed.paths,
    })
}

/// A `--changed-file` path: relative to `cwd` when it exists there, else to the repository.
fn explicit_file(path: &Path, cwd: &Path, repository: &Path) -> PathBuf {
    let from_cwd = cwd.join(path);
    let candidate = if from_cwd.exists() {
        from_cwd
    } else {
        repository.join(path)
    };
    std::fs::canonicalize(&candidate).unwrap_or(candidate)
}

fn fallback_reasons(files: &[PathBuf], deleted: &[PathBuf]) -> Vec<String> {
    let mut reasons = Vec::new();
    if deleted.iter().any(|path| is_source(path)) {
        reasons.push(
            "a tracked source file was deleted; prior dependency edges are unavailable".into(),
        );
    }
    reasons.extend(
        files
            .iter()
            .filter_map(|path| global_trigger(path).map(str::to_string)),
    );
    reasons.sort();
    reasons.dedup();
    reasons
}

pub(super) fn global_trigger(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    if matches!(
        name,
        "package.json"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "pnpm-lock.yml"
            | "yarn.lock"
            | "bun.lock"
            | "bun.lockb"
            | "pyproject.toml"
            | "pytest.ini"
            | "setup.cfg"
            | "tox.ini"
            | "sensez.toml"
    ) {
        return Some("a dependency or test-discovery manifest changed");
    }
    if name == "conftest.py" {
        return Some("shared pytest fixtures changed");
    }
    let lower = name.to_ascii_lowercase();
    if (lower.starts_with("vitest.") || lower.starts_with("jest.")) && lower.contains("config") {
        return Some("test-runner configuration changed");
    }
    if lower.starts_with("tsconfig") && lower.ends_with(".json") {
        return Some("TypeScript module resolution changed");
    }
    if lower.starts_with(".env.test")
        || matches!(
            lower.as_str(),
            "setuptests.js"
                | "setuptests.ts"
                | "setup-tests.js"
                | "setup-tests.ts"
                | "test-setup.js"
                | "test-setup.ts"
                | "test_setup.py"
        )
    {
        return Some("shared test environment setup changed");
    }
    None
}

/// Language of a source file whose tests reflexez can select: its profile names a test runner.
pub(super) fn source_language(path: &Path) -> Option<Language> {
    registry::parse_for_path(path)
        .map(|profile| profile.info())
        .filter(|info| !info.test_runners.is_empty())
        .map(|info| info.language)
}

pub(super) fn is_source(path: &Path) -> bool {
    source_language(path).is_some()
}

#[cfg(test)]
#[path = "changes_tests.rs"]
mod tests;

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
    staged: bool,
    explicit: &[PathBuf],
) -> Result<ChangeScope> {
    if !explicit.is_empty() {
        let repository = crate::diff::git::repository_root(root)?;
        let files: Vec<_> = explicit
            .iter()
            .map(|path| {
                let candidate = if path.is_absolute() {
                    path.clone()
                } else {
                    repository.join(path)
                };
                std::fs::canonicalize(&candidate).unwrap_or(candidate)
            })
            .collect();
        return Ok(ChangeScope {
            fallback_reasons: fallback_reasons(&files, &[]),
            repository,
            files,
        });
    }
    let changed = crate::diff::git::changed_paths(root, base, staged)?;
    Ok(ChangeScope {
        repository: changed.root,
        fallback_reasons: fallback_reasons(&changed.paths, &changed.deleted),
        files: changed.paths,
    })
}

fn fallback_reasons(files: &[PathBuf], deleted: &[PathBuf]) -> Vec<String> {
    let mut reasons = Vec::new();
    if !deleted.is_empty() {
        reasons.push("a tracked file was deleted; prior dependency edges are unavailable".into());
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

pub(super) fn is_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("py" | "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts")
    )
}

pub(super) fn is_declaration_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            [".d.ts", ".d.mts", ".d.cts"]
                .iter()
                .any(|suffix| name.ends_with(suffix))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_configuration_and_shared_setup_triggers() {
        for path in [
            "sensez.toml",
            "pyproject.toml",
            "vitest.config.ts",
            "tests/test_setup.py",
            ".env.test.local",
        ] {
            assert!(global_trigger(Path::new(path)).is_some(), "{path}");
        }
        assert!(global_trigger(Path::new("src/setup.ts")).is_none());
    }
}

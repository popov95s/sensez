//! Shared glob-set compilation for exclude/entry-point patterns.
//!
//! One implementation for every pillar (crawler, dead code, duplication,
//! smells). User-configured globs are validated at config-load time so a typo'd
//! exclude fails loudly instead of silently changing scan scope.

use anyhow::{anyhow, Result};
use globset::{GlobBuilder, GlobSet};

/// Validate every glob in a user-facing config field.
pub fn validate_patterns(label: &str, patterns: &[String]) -> Result<()> {
    for pattern in patterns {
        GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .map(|_| ())
            .map_err(|err| anyhow!("invalid glob in {label} ({pattern:?}): {err}"))?;
    }
    Ok(())
}

/// Compile `patterns` into a [`GlobSet`].
pub fn build_globset(patterns: &[impl AsRef<str>]) -> Result<GlobSet> {
    let mut builder = GlobSet::builder();
    for pattern in patterns.iter().map(AsRef::as_ref) {
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .map_err(|err| anyhow!("invalid glob pattern ({pattern:?}): {err}"))?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|err| anyhow!("glob patterns failed to compile as a set: {err}"))
}

/// Compile patterns that config loading already validated; empty set if that ever fails.
pub fn compile_validated(patterns: &[impl AsRef<str>]) -> GlobSet {
    build_globset(patterns).unwrap_or_else(|err| {
        debug_assert!(false, "unvalidated glob reached compile_validated: {err}");
        GlobSet::empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_globs_match() {
        let set = build_globset(&["**/tests/**".to_string()]).unwrap();
        assert!(set.is_match("src/tests/x.py"));
        assert!(!set.is_match("src/main.py"));
        let python_tests = build_globset(&["**/test_*.py".to_string()]).unwrap();
        assert!(python_tests.is_match("src/test_api.py"));
        assert!(!python_tests.is_match("src/test_helpers/example.py"));
    }

    #[test]
    fn invalid_globs_fail_validation() {
        let err = validate_patterns("exclude", &["[invalid".to_string()]).unwrap_err();
        assert!(err.to_string().contains("invalid glob in exclude"));
    }
}

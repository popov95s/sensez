//! Shared glob-set compilation for exclude/entry-point patterns.
//!
//! One implementation for every pillar (crawler, dead code, duplication,
//! smells). User-configured globs are validated at config-load time so a typo'd
//! exclude fails loudly instead of silently changing scan scope.

use anyhow::{anyhow, Result};
use globset::{Glob, GlobSet};

/// Validate every glob in a user-facing config field.
pub fn validate_patterns(label: &str, patterns: &[String]) -> Result<()> {
    for pattern in patterns {
        Glob::new(pattern)
            .map(|_| ())
            .map_err(|err| anyhow!("invalid glob in {label} ({pattern:?}): {err}"))?;
    }
    Ok(())
}

/// Compile already-validated `patterns` into a [`GlobSet`].
pub fn build_globset(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSet::builder();
    for pattern in patterns {
        let glob = Glob::new(pattern).expect("glob patterns are validated during config load");
        builder.add(glob);
    }
    builder
        .build()
        .expect("validated glob patterns should compile as a set")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_globs_match() {
        let set = build_globset(&["**/tests/**".to_string()]);
        assert!(set.is_match("src/tests/x.py"));
        assert!(!set.is_match("src/main.py"));
    }

    #[test]
    fn invalid_globs_fail_validation() {
        let err = validate_patterns("exclude", &["[invalid".to_string()]).unwrap_err();
        assert!(err.to_string().contains("invalid glob in exclude"));
    }
}

//! JS/TS test-support layouts that dead code still treats as live importers.

pub(crate) const TEST_SUPPORT: &[&str] = &[
    "**/__mocks__/**",
    "**/__fixtures__/**",
    "**/e2e/**",
    "**/test-utils/**",
    "**/*.stories.{ts,tsx,js,jsx}",
];

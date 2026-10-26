use serde::{Deserialize, Serialize};

/// Parsed files a source-level pillar left out, by reason.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkipCounts {
    pub config_exclude: usize,
    pub tests: usize,
    pub generated: usize,
}

impl SkipCounts {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

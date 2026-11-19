//! `[self_improvement]` — opt-out for Sensez' local-only learning data.

use serde::{Deserialize, Serialize};

/// Controls Sensez' LOCAL-ONLY self-improvement data under `.sensez/local-metrics/`.
#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct SelfImprovement {
    pub enabled: bool,
}

impl Default for SelfImprovement {
    fn default() -> Self {
        SelfImprovement { enabled: true }
    }
}

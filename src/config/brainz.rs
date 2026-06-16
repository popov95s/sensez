//! `[self_improvement]` — opt-out for Sensez' local-only learning data.
//!
//! "Local" is deliberate: nothing is ever transmitted anywhere regardless of
//! this setting (see SECURITY.md). The data lives under `.sensez/local-metrics/`
//! and lets sensez learn from a working session — which findings you fix, which
//! you dismiss as false positives — to tune what it surfaces and to show its
//! value via `brainz_report`.

use serde::Deserialize;

/// Controls Sensez' LOCAL-ONLY self-improvement data under
/// `.sensez/local-metrics/` (scans run, findings resolved, searches served,
/// triage verdicts). `enabled = false` stops even the on-disk recording — no
/// events log, no totals, no fingerprint baselines — and `brainz_report`
/// answers "disabled".
#[derive(Debug, Clone, Hash, Deserialize)]
#[serde(default)]
pub struct SelfImprovement {
    pub enabled: bool,
}

impl Default for SelfImprovement {
    fn default() -> Self {
        SelfImprovement { enabled: true }
    }
}

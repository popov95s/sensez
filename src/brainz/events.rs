//! Local-only usage metric types.

use super::verdict::TriageVerdict;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How a recorded scan was triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// An explicit `noze_sniff` tool call.
    Tool,
    /// The end-of-turn Stop-hook quality gate (diff-scoped).
    Gate,
    /// A direct CLI invocation (reserved; not yet wired).
    Cli,
}

impl Origin {
    /// Stable key for `scans_by_origin` aggregation.
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Tool => "tool",
            Origin::Gate => "gate",
            Origin::Cli => "cli",
        }
    }
}

/// Resolved findings for one detector in a single diff.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolved {
    pub count: u64,
    pub secs_total: u64,
}

impl Resolved {
    /// Fold another detector tally into this one.
    pub fn merge(&mut self, other: &Resolved) {
        self.count += other.count;
        self.secs_total += other.secs_total;
    }
}

/// Outcome tally key; stored in `totals.json` as the string `"verdict:detector"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutcomeKey {
    pub verdict: TriageVerdict,
    pub detector: String,
}

impl OutcomeKey {
    pub fn new(verdict: TriageVerdict, detector: impl Into<String>) -> Self {
        Self {
            verdict,
            detector: detector.into(),
        }
    }
}

impl Serialize for OutcomeKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}:{}", self.verdict, self.detector))
    }
}

impl<'de> Deserialize<'de> for OutcomeKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        text.split_once(':')
            .and_then(|(verdict, detector)| {
                TriageVerdict::from_name(verdict).map(|verdict| OutcomeKey::new(verdict, detector))
            })
            .ok_or_else(|| serde::de::Error::custom(format!("invalid outcome key: {text}")))
    }
}

/// One recorded server interaction (a row in append-only `events.jsonl`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// A `noze_sniff` tool call.
    Scan {
        ts: u64,
        session: String,
        /// Git branch at record time (empty when not a git repo / detached).
        branch: String,
        ms: u64,
        /// What triggered the scan (gate vs. explicit call).
        origin: Origin,
        /// Detector (`pillar/<kind>`) → findings reported this scan.
        reported: BTreeMap<String, u64>,
        /// Detector → findings that vanished since the last scan.
        resolved: BTreeMap<String, Resolved>,
        /// Detector → findings previously resolved that came back.
        reintroduced: BTreeMap<String, Resolved>,
        /// Files analyzed and total source lines (size denominators for health).
        files: u64,
        loc: u64,
        /// Hash of the effective config, for detecting config churn.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        config_hash: Option<u64>,
    },
    /// An `eyez_search_docs` call.
    Search {
        ts: u64,
        session: String,
        branch: String,
        ms: u64,
        query_len: u64,
        hits: u64,
        top_score: f32,
        first_on_repo: bool,
        /// Snippet bytes actually returned vs.
        bytes_returned: u64,
        file_bytes_referenced: u64,
    },
    /// Automatic fix recapture.
    AutoResolve {
        ts: u64,
        session: String,
        branch: String,
        /// Detector → resolved count + summed time-to-resolution.
        resolved: BTreeMap<String, Resolved>,
        /// Detector → reintroductions + summed resolved-interval.
        reintroduced: BTreeMap<String, Resolved>,
    },
    /// The end-of-turn gate blocked, listing the fingerprints it flagged.
    GateBlock {
        ts: u64,
        session: String,
        branch: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        scope: Option<String>,
        fingerprints: Vec<String>,
    },
    /// A user-triage outcome.
    Outcome {
        ts: u64,
        session: String,
        branch: String,
        #[serde(rename = "pillar")]
        detector: String,
        #[serde(rename = "action")]
        verdict: TriageVerdict,
        count: u64,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        detail: Option<String>,
    },
}

impl Event {
    /// Record timestamp (unix seconds) — used to window the event log.
    pub fn ts(&self) -> u64 {
        match self {
            Event::Scan { ts, .. }
            | Event::Search { ts, .. }
            | Event::AutoResolve { ts, .. }
            | Event::GateBlock { ts, .. }
            | Event::Outcome { ts, .. } => *ts,
        }
    }
}

mod totals;
pub use totals::Totals;
#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;

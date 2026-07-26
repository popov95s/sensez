//! Local-only usage metric types. Everything here serializes to plain JSON on
//! the user's own disk under `.sensez/local-metrics/` — nothing is ever exported.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How a recorded scan was triggered. Lets reports isolate the Stop-hook
/// gate's effect (issues caught on the edit that introduced them) from
/// explicit `noze_sniff` calls.
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

/// Resolved findings for one detector in a single diff: how many vanished and
/// the summed age (seconds from `first_seen` to disappearance) of those
/// findings, so reports can compute a mean time-to-resolution per detector.
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutcomeKey {
    pub verdict: String,
    pub detector: String,
}

impl OutcomeKey {
    /// Create a new outcome key.
    pub fn new(verdict: impl Into<String>, detector: impl Into<String>) -> Self {
        Self {
            verdict: verdict.into(),
            detector: detector.into(),
        }
    }
}

impl Serialize for OutcomeKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as "verdict:detector" for JSON compatibility
        serializer.serialize_str(&format!("{}:{}", self.verdict, self.detector))
    }
}

impl<'de> Deserialize<'de> for OutcomeKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let (verdict, detector) = s
            .split_once(':')
            .ok_or_else(|| serde::de::Error::custom(format!("invalid outcome key format: {s}")))?;
        Ok(OutcomeKey::new(verdict, detector))
    }
}

/// One recorded server interaction (a row in append-only `events.jsonl`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// A `noze_sniff` tool call: what each detector reported, and which previous
    /// findings disappeared since the last full scan of this repo (= likely
    /// fixed after we reported them).
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
        /// Detector → findings that vanished since the last scan, with their
        /// summed time-to-resolution.
        resolved: BTreeMap<String, Resolved>,
        /// Detector → findings previously resolved that came back (a fix that
        /// did not stick), with the summed interval they stayed resolved.
        reintroduced: BTreeMap<String, Resolved>,
        /// Files analyzed and total source lines (size denominators for health).
        files: u64,
        loc: u64,
        /// Hash of the effective config, for detecting config churn. `None`
        /// when the config could not be loaded.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        config_hash: Option<u64>,
    },
    /// An `eyez_search_docs` call. `first_on_repo` marks a first-ever search of this
    /// codebase (first-time orientation sensez served).
    Search {
        ts: u64,
        session: String,
        branch: String,
        ms: u64,
        query_len: u64,
        hits: u64,
        top_score: f32,
        first_on_repo: bool,
        /// Snippet bytes actually returned vs. the total size of the files
        /// those snippets point into — context the agent did NOT have to read.
        bytes_returned: u64,
        file_bytes_referenced: u64,
    },
    /// Automatic fix recapture: the server itself re-ran the pipeline after
    /// sources changed and found previously-reported findings gone. No agent
    /// cooperation involved.
    AutoResolve {
        ts: u64,
        session: String,
        branch: String,
        /// Detector → resolved count + summed time-to-resolution.
        resolved: BTreeMap<String, Resolved>,
        /// Detector → reintroductions + summed resolved-interval.
        reintroduced: BTreeMap<String, Resolved>,
    },
    /// The end-of-turn gate blocked, listing the fingerprints it flagged. Lets
    /// the report compute block→fix conversion: of findings the gate caught at
    /// edit time, how many were gone by the next full scan vs. escaped open.
    GateBlock {
        ts: u64,
        session: String,
        branch: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        scope: Option<String>,
        fingerprints: Vec<String>,
    },
    /// A user-triage outcome: the user adjudicated a finding (debt /
    /// false_positive) via `brainz_triage`. `pillar` carries the detector id
    /// so per-detector precision is derivable. Never produced by the model.
    Outcome {
        ts: u64,
        session: String,
        branch: String,
        pillar: String,
        action: String,
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

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

/// Running aggregates for one repo. Persisted as `totals.json` and also kept
/// per-session in memory; both are built by absorbing [`Event`]s.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Totals {
    pub first_used: u64,
    pub last_used: u64,
    pub scans: u64,
    pub searches: u64,
    /// Searches that were the first ever on their repo.
    pub first_searches: u64,
    /// Searches that returned no hits (a miss the user had to work around).
    pub searches_zero_hit: u64,
    /// `"<verdict>:<detector>"` → user-triage outcome count.
    pub outcomes: BTreeMap<String, u64>,
    /// Sum of (referenced file bytes − returned snippet bytes) over searches.
    pub est_context_bytes_saved: u64,
    /// Detector (`pillar/<kind>`) → findings reported by the latest scan.
    pub reported_by_detector: BTreeMap<String, u64>,
    /// Detector → resolved count + summed time-to-resolution (seconds).
    pub resolved_by_detector: BTreeMap<String, Resolved>,
    /// Detector → previously-resolved findings that came back,
    /// with the summed interval they stayed resolved.
    pub reintroduced_by_detector: BTreeMap<String, Resolved>,
    /// Scan origin (`tool` | `gate` | `cli`) → scans recorded.
    pub scans_by_origin: BTreeMap<String, u64>,
    /// Summed scan wall-time (ms), analyzed files, and source lines, for
    /// ms-per-file / ms-per-kloc throughput health.
    pub scan_ms_total: u64,
    pub scan_files_total: u64,
    pub scan_loc_total: u64,
    /// Times the effective config changed between consecutive scans, and the
    /// most recent config hash (the comparison anchor — not a reported metric).
    pub config_changes: u64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_config_hash: Option<u64>,
    /// Times the end-of-turn gate blocked (caught findings on the edit).
    pub gate_blocks: u64,
}

impl Totals {
    /// Fold one event into the running aggregates.
    pub fn absorb(&mut self, event: &Event) {
        let ts = event.ts();
        if self.first_used == 0 {
            self.first_used = ts;
        }
        self.last_used = self.last_used.max(ts);
        match event {
            Event::Scan {
                ms,
                origin,
                reported,
                resolved,
                reintroduced,
                files,
                loc,
                config_hash,
                ..
            } => {
                self.scans += 1;
                self.scan_ms_total += ms;
                self.scan_files_total += files;
                self.scan_loc_total += loc;
                *self
                    .scans_by_origin
                    .entry(origin.as_str().to_string())
                    .or_default() += 1;
                self.reported_by_detector = reported.clone();
                if let Some(hash) = config_hash {
                    if self.last_config_hash.is_some_and(|prev| prev != *hash) {
                        self.config_changes += 1;
                    }
                    self.last_config_hash = Some(*hash);
                }
                self.absorb_diff(resolved, reintroduced);
            }
            Event::Search {
                hits,
                first_on_repo,
                bytes_returned,
                file_bytes_referenced,
                ..
            } => {
                self.searches += 1;
                if *first_on_repo {
                    self.first_searches += 1;
                }
                if *hits == 0 {
                    self.searches_zero_hit += 1;
                }
                self.est_context_bytes_saved +=
                    file_bytes_referenced.saturating_sub(*bytes_returned);
            }
            Event::AutoResolve {
                resolved,
                reintroduced,
                ..
            } => self.absorb_diff(resolved, reintroduced),
            Event::GateBlock { .. } => self.gate_blocks += 1,
            Event::Outcome {
                pillar,
                action,
                count,
                ..
            } => {
                *self
                    .outcomes
                    .entry(format!("{action}:{pillar}"))
                    .or_default() += count;
            }
        }
    }

    /// Fold per-detector resolved tallies (count + time-to-resolution) and
    /// reintroduction counts into the running aggregates. Shared by `Scan` and
    /// `AutoResolve`.
    fn absorb_diff(
        &mut self,
        resolved: &BTreeMap<String, Resolved>,
        reintroduced: &BTreeMap<String, Resolved>,
    ) {
        for (detector, r) in resolved {
            self.resolved_by_detector
                .entry(detector.clone())
                .or_default()
                .merge(r);
        }
        for (detector, r) in reintroduced {
            self.reintroduced_by_detector
                .entry(detector.clone())
                .or_default()
                .merge(r);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absorb_aggregates_each_event_kind() {
        let mut t = Totals::default();
        t.absorb(&Event::Scan {
            ts: 10,
            session: "s".into(),
            branch: "main".into(),
            ms: 5,
            origin: Origin::Gate,
            reported: BTreeMap::from([("dead_code/function".into(), 3)]),
            resolved: BTreeMap::from([(
                "dead_code/function".into(),
                Resolved {
                    count: 1,
                    secs_total: 3600,
                },
            )]),
            reintroduced: BTreeMap::from([(
                "smells/god_module".into(),
                Resolved {
                    count: 2,
                    secs_total: 200,
                },
            )]),
            files: 120,
            loc: 9000,
            config_hash: Some(42),
        });
        t.absorb(&Event::Search {
            ts: 20,
            session: "s".into(),
            branch: "main".into(),
            ms: 5,
            query_len: 12,
            hits: 4,
            top_score: 0.7,
            first_on_repo: true,
            bytes_returned: 100,
            file_bytes_referenced: 5000,
        });
        t.absorb(&Event::Outcome {
            ts: 30,
            session: "s".into(),
            branch: "main".into(),
            pillar: "duplication".into(),
            action: "fixed".into(),
            count: 2,
            detail: None,
        });

        assert_eq!((t.first_used, t.last_used), (10, 30));
        assert_eq!((t.scans, t.searches, t.first_searches), (1, 1, 1));
        assert_eq!(t.outcomes["fixed:duplication"], 2);
        assert_eq!(t.est_context_bytes_saved, 4900);
        assert_eq!(t.scans_by_origin["gate"], 1);
        assert_eq!(t.reported_by_detector["dead_code/function"], 3);
        let ttr = &t.resolved_by_detector["dead_code/function"];
        assert_eq!((ttr.count, ttr.secs_total), (1, 3600));
        assert_eq!(t.reintroduced_by_detector["smells/god_module"].count, 2);
        assert_eq!(
            (t.scan_ms_total, t.scan_files_total, t.scan_loc_total),
            (5, 120, 9000)
        );
        // One scan → a config hash is anchored but no change is counted yet.
        assert_eq!((t.config_changes, t.last_config_hash), (0, Some(42)));
    }
}

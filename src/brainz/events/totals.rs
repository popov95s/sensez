use super::{Event, OutcomeKey, Resolved};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// User-triage outcome counts keyed by verdict and detector.
    pub outcomes: BTreeMap<OutcomeKey, u64>,
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
                    .entry(OutcomeKey::new(action, pillar))
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

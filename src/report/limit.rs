//! Top-N truncation shared by the CLI `--max` flag and the MCP `limit` argument.

use super::{AnalysisReport, SmellFinding};
use std::collections::BTreeMap;

/// How smells are capped next to the per-pillar cap; zero leaves them uncapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmellCap {
    /// At most N smells in total, in ranked order.
    Total(usize),
    /// At most N smells of each kind, grouped by kind.
    PerKind(usize),
}

/// Top-N caps for one report; a zero cap leaves that part uncapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitPolicy {
    pub pillar_max: usize,
    pub smells: SmellCap,
}

impl LimitPolicy {
    /// The same cap for every pillar, smells counted in total.
    pub fn flat(max: usize) -> Self {
        Self {
            pillar_max: max,
            smells: SmellCap::Total(max),
        }
    }
}

/// Truncate each pillar to its top-ranked findings under `policy`.
pub fn limit(report: &mut AnalysisReport, policy: LimitPolicy) {
    if policy.pillar_max > 0 {
        report.cycles.truncate(policy.pillar_max);
        report.dead_code.truncate(policy.pillar_max);
        report.boundaries.truncate(policy.pillar_max);
        report.duplication.truncate(policy.pillar_max);
    }
    match policy.smells {
        SmellCap::Total(0) | SmellCap::PerKind(0) => {}
        SmellCap::Total(max) => report.smells.truncate(max),
        SmellCap::PerKind(max) => limit_smells_by_kind(&mut report.smells, max),
    }
}

fn limit_smells_by_kind(smells: &mut Vec<SmellFinding>, max: usize) {
    let mut seen: BTreeMap<&'static str, usize> = BTreeMap::new();
    smells.retain(|finding| {
        let count = seen.entry(finding.kind.as_str()).or_default();
        let keep = *count < max;
        *count += 1;
        keep
    });
    smells.sort_by(|a, b| {
        a.kind
            .as_str()
            .cmp(b.kind.as_str())
            .then(a.action.cmp(&b.action))
            .then(b.metric.cmp(&a.metric))
    });
}

#[cfg(test)]
#[path = "limit_tests.rs"]
mod tests;

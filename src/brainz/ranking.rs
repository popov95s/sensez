//! Precision-aware presentation ranking for findings.

use super::fingerprint;
use super::{hub, report, store};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn regressions(root: &Path, branch: Option<&str>, report: &Value) -> Vec<String> {
    let Some(branch) = branch else {
        return Vec::new();
    };
    let history = store::load_resolved_history(root, branch);
    if history.is_empty() {
        return Vec::new();
    }
    fingerprint::fingerprints(report)
        .values()
        .flatten()
        .filter(|p| history.contains_key(&p.key()))
        .map(|p| p.label.to_string())
        .collect()
}

pub fn rank_by_precision(root: &Path, report: &mut crate::report::AnalysisReport) {
    let noisy = noisy_detectors(root);
    if noisy.is_empty() {
        return;
    }
    let Ok(value) = serde_json::to_value(&*report) else {
        return;
    };
    let prints = fingerprint::fingerprints(&value);
    fingerprint::partition_by_fingerprint(
        &mut report.cycles,
        prints.get(&fingerprint::Pillar::Cycles),
        |p| noisy.contains(&p.class.to_string()),
    );
    fingerprint::partition_by_fingerprint(
        &mut report.dead_code,
        prints.get(&fingerprint::Pillar::DeadCode),
        |p| noisy.contains(&p.class.to_string()),
    );
    fingerprint::partition_by_fingerprint(
        &mut report.boundaries,
        prints.get(&fingerprint::Pillar::Boundaries),
        |p| noisy.contains(&p.class.to_string()),
    );
    fingerprint::partition_by_fingerprint(
        &mut report.duplication,
        prints.get(&fingerprint::Pillar::Duplication),
        |p| noisy.contains(&p.class.to_string()),
    );
    fingerprint::partition_by_fingerprint(
        &mut report.smells,
        prints.get(&fingerprint::Pillar::Smells),
        |p| noisy.contains(&p.class.to_string()),
    );
}

fn noisy_detectors(root: &Path) -> BTreeSet<String> {
    hub::cached_noisy(root, || {
        report::low_precision_detectors(&store::load_totals(root))
    })
}

#[cfg(test)]
#[path = "ranking_tests.rs"]
mod tests;

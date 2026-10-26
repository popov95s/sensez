//! Finding identities the gate remembers, and pruning a report by them.

use super::fingerprint;
use crate::report::AnalysisReport;
use std::collections::{BTreeMap, BTreeSet};

/// Fingerprint key to human label for every finding in `report`.
pub fn finding_identities(report: &AnalysisReport) -> BTreeMap<String, String> {
    let Ok(value) = serde_json::to_value(report) else {
        return BTreeMap::new();
    };
    fingerprint::fingerprints(&value)
        .values()
        .flatten()
        .map(|print| (print.key(), print.label.to_string()))
        .collect()
}

/// Drop findings whose identity is in `blocked`; returns how many remain.
pub fn retain_unseen_gate_findings(
    report: &mut AnalysisReport,
    blocked: &BTreeSet<String>,
) -> usize {
    if blocked.is_empty() {
        return report.finding_count();
    }
    let Ok(value) = serde_json::to_value(&*report) else {
        return report.finding_count();
    };
    let prints = fingerprint::fingerprints(&value);
    let unseen = |print: &fingerprint::Print| !blocked.contains(&print.key());
    fingerprint::retain_by_fingerprint(
        &mut report.cycles,
        prints.get(&fingerprint::Pillar::Cycles),
        unseen,
    );
    fingerprint::retain_by_fingerprint(
        &mut report.dead_code,
        prints.get(&fingerprint::Pillar::DeadCode),
        unseen,
    );
    fingerprint::retain_by_fingerprint(
        &mut report.boundaries,
        prints.get(&fingerprint::Pillar::Boundaries),
        unseen,
    );
    fingerprint::retain_by_fingerprint(
        &mut report.duplication,
        prints.get(&fingerprint::Pillar::Duplication),
        unseen,
    );
    fingerprint::retain_by_fingerprint(
        &mut report.smells,
        prints.get(&fingerprint::Pillar::Smells),
        unseen,
    );
    report.meta.glossary = crate::noze::glossary::for_report(report);
    report.finding_count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{ActionLevel, Confidence, DeadCodeFinding};
    use crate::spine::ir::SymbolKind;
    use std::path::PathBuf;

    #[test]
    fn previously_blocked_identity_is_pruned_by_fingerprint() {
        let mut first = report_with(&[("alpha", 4)]);
        let blocked: BTreeSet<String> = finding_identities(&first).into_keys().collect();

        let mut next = report_with(&[("alpha", 40), ("beta", 12)]);
        assert_eq!(retain_unseen_gate_findings(&mut next, &blocked), 1);
        assert_eq!(next.dead_code[0].symbol, "beta");

        first.dead_code[0].line = 400;
        assert_eq!(
            retain_unseen_gate_findings(&mut first, &blocked),
            0,
            "line drift keeps the same finding identity"
        );
    }

    fn report_with(symbols: &[(&str, usize)]) -> AnalysisReport {
        AnalysisReport {
            dead_code: symbols
                .iter()
                .map(|(symbol, line)| DeadCodeFinding {
                    action: ActionLevel::Advisory,
                    module: "sample".to_string(),
                    symbol: (*symbol).to_string(),
                    kind: SymbolKind::Function,
                    confidence: Confidence::High,
                    file: PathBuf::from("sample.py"),
                    line: *line,
                    reason: String::new(),
                })
                .collect(),
            ..AnalysisReport::default()
        }
    }
}

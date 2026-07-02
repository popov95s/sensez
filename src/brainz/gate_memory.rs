//! Remember which finding identities have already blocked the gate.

use super::events::Event;
use super::{fingerprint, hub, store};
use crate::report::AnalysisReport;
use std::collections::BTreeSet;

pub fn retain_unseen_gate_findings(root: &std::path::Path, report: &mut AnalysisReport) -> usize {
    let blocked = blocked_fingerprints(root);
    if blocked.is_empty() {
        return finding_count(report);
    }
    let Ok(value) = serde_json::to_value(&*report) else {
        return finding_count(report);
    };
    let prints = fingerprint::fingerprints(&value);
    retain_unblocked(&mut report.cycles, prints.get("cycles"), &blocked);
    retain_unblocked(&mut report.dead_code, prints.get("dead_code"), &blocked);
    retain_unblocked(&mut report.boundaries, prints.get("boundaries"), &blocked);
    retain_unblocked(&mut report.duplication, prints.get("duplication"), &blocked);
    retain_unblocked(&mut report.smells, prints.get("smells"), &blocked);
    report.meta.glossary = crate::noze::glossary::for_report(report);
    finding_count(report)
}

fn blocked_fingerprints(root: &std::path::Path) -> BTreeSet<String> {
    let branch = hub::branch_label(root);
    store::load_events(root)
        .into_iter()
        .filter_map(|event| match event {
            Event::GateBlock {
                branch: event_branch,
                fingerprints,
                ..
            } if event_branch == branch => Some(fingerprints),
            _ => None,
        })
        .flatten()
        .collect()
}

fn retain_unblocked<T>(
    items: &mut Vec<T>,
    prints: Option<&Vec<fingerprint::Print>>,
    blocked: &BTreeSet<String>,
) {
    let Some(prints) = prints else {
        return;
    };
    let mut i = 0;
    items.retain(|_| {
        let keep = prints
            .get(i)
            .map(|p| !blocked.contains(&format!("{:x}", p.hash)))
            .unwrap_or(true);
        i += 1;
        keep
    });
}

fn finding_count(report: &AnalysisReport) -> usize {
    report.duplication.len()
        + report.dead_code.len()
        + report.cycles.len()
        + report.boundaries.len()
        + report.smells.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{ActionLevel, Confidence, DeadCodeFinding};
    use crate::spine::ir::SymbolKind;
    use std::path::{Path, PathBuf};

    #[test]
    fn previously_blocked_identity_is_pruned_by_fingerprint() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let mut first = report_with(root, &[("alpha", 4)]);
        let value = serde_json::to_value(&first).unwrap();
        crate::brainz::record_gate_block(root, &value);
        crate::brainz::flush();

        let mut next = report_with(root, &[("alpha", 40), ("beta", 12)]);
        assert_eq!(retain_unseen_gate_findings(root, &mut next), 1);
        assert_eq!(next.dead_code.len(), 1);
        assert_eq!(next.dead_code[0].symbol, "beta");

        first.dead_code[0].line = 400;
        assert_eq!(
            retain_unseen_gate_findings(root, &mut first),
            0,
            "line drift keeps the same finding identity"
        );
    }

    fn report_with(root: &Path, symbols: &[(&str, usize)]) -> AnalysisReport {
        AnalysisReport {
            dead_code: symbols
                .iter()
                .map(|(symbol, line)| DeadCodeFinding {
                    action: ActionLevel::Advisory,
                    module: "sample".to_string(),
                    symbol: (*symbol).to_string(),
                    kind: SymbolKind::Function,
                    confidence: Confidence::High,
                    file: PathBuf::from(root).join("sample.py"),
                    line: *line,
                    reason: String::new(),
                })
                .collect(),
            ..AnalysisReport::default()
        }
    }
}

//! Remember which finding identities have already blocked the gate.

use super::events::Event;
use super::{fingerprint, store};
use crate::report::AnalysisReport;
use std::collections::BTreeSet;

pub fn retain_unseen_gate_findings(
    root: &std::path::Path,
    scope: Option<&str>,
    report: &mut AnalysisReport,
) -> usize {
    let blocked = blocked_fingerprints(root, scope);
    if blocked.is_empty() {
        return finding_count(report);
    }
    let Ok(value) = serde_json::to_value(&*report) else {
        return finding_count(report);
    };
    let prints = fingerprint::fingerprints(&value);
    fingerprint::retain_by_fingerprint(
        &mut report.cycles,
        prints.get(&fingerprint::Namespace::Cycles),
        |p| !blocked.contains(&p.key()),
    );
    fingerprint::retain_by_fingerprint(
        &mut report.dead_code,
        prints.get(&fingerprint::Namespace::DeadCode),
        |p| !blocked.contains(&p.key()),
    );
    fingerprint::retain_by_fingerprint(
        &mut report.boundaries,
        prints.get(&fingerprint::Namespace::Boundaries),
        |p| !blocked.contains(&p.key()),
    );
    fingerprint::retain_by_fingerprint(
        &mut report.duplication,
        prints.get(&fingerprint::Namespace::Duplication),
        |p| !blocked.contains(&p.key()),
    );
    fingerprint::retain_by_fingerprint(
        &mut report.smells,
        prints.get(&fingerprint::Namespace::Smells),
        |p| !blocked.contains(&p.key()),
    );
    report.meta.glossary = crate::noze::glossary::for_report(report);
    finding_count(report)
}

fn blocked_fingerprints(root: &std::path::Path, scope: Option<&str>) -> BTreeSet<String> {
    store::load_events(root)
        .into_iter()
        .filter_map(|event| match event {
            Event::GateBlock {
                scope: event_scope,
                fingerprints,
                ..
            } if event_scope.as_deref() == scope => Some(fingerprints),
            _ => None,
        })
        .flatten()
        .collect()
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
        crate::brainz::record_gate_block(root, None, &value);
        crate::brainz::flush();

        let mut next = report_with(root, &[("alpha", 40), ("beta", 12)]);
        assert_eq!(retain_unseen_gate_findings(root, None, &mut next), 1);
        assert_eq!(next.dead_code.len(), 1);
        assert_eq!(next.dead_code[0].symbol, "beta");

        first.dead_code[0].line = 400;
        assert_eq!(
            retain_unseen_gate_findings(root, None, &mut first),
            0,
            "line drift keeps the same finding identity"
        );
    }

    #[test]
    fn blocked_findings_are_isolated_by_session_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let first = report_with(root, &[("alpha", 4)]);
        let value = serde_json::to_value(&first).unwrap();
        crate::brainz::record_gate_block(root, Some("session:one"), &value);
        crate::brainz::flush();

        let mut other_session = report_with(root, &[("alpha", 4)]);
        assert_eq!(
            retain_unseen_gate_findings(root, Some("session:two"), &mut other_session),
            1
        );

        let mut original_session = report_with(root, &[("alpha", 4)]);
        assert_eq!(
            retain_unseen_gate_findings(root, Some("session:one"), &mut original_session),
            0
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

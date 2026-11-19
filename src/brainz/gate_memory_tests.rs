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

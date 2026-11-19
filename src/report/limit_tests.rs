use super::*;
use crate::report::{ActionLevel, Severity, SmellKind};

fn smell(kind: SmellKind, metric: u32) -> SmellFinding {
    SmellFinding {
        action: ActionLevel::Warning,
        kind,
        message: String::new(),
        file: "a.py".into(),
        line: 1,
        end_line: 1,
        lines: Vec::new(),
        symbol: "f".into(),
        severity: Severity::Warning,
        metric,
        threshold: 0,
        reason: String::new(),
    }
}

fn report() -> AnalysisReport {
    AnalysisReport {
        smells: vec![
            smell(SmellKind::LongFunction, 9),
            smell(SmellKind::LongFunction, 8),
            smell(SmellKind::DeepNesting, 7),
        ],
        ..AnalysisReport::default()
    }
}

fn metrics(report: &AnalysisReport) -> Vec<u32> {
    report.smells.iter().map(|smell| smell.metric).collect()
}

#[test]
fn total_cap_keeps_the_top_ranked_smells() {
    let mut report = report();
    limit(&mut report, LimitPolicy::flat(2));
    assert_eq!(metrics(&report), [9, 8]);
}

#[test]
fn per_kind_cap_keeps_the_top_smells_of_each_kind() {
    let mut report = report();
    let policy = LimitPolicy {
        pillar_max: 1,
        smells: SmellCap::PerKind(1),
    };
    limit(&mut report, policy);
    assert_eq!(metrics(&report), [7, 9]);
}

//! Precision-aware presentation ranking for findings.

use super::fingerprint;
use super::{hub, report, store};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn regressions(root: &Path, report: &Value) -> Vec<String> {
    let Some(branch) = hub::branch_key(root) else {
        return Vec::new();
    };
    let history = store::load_resolved_history(root, &branch);
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
        prints.get(&fingerprint::Namespace::Cycles),
        |p| noisy.contains(&p.class.to_string()),
    );
    fingerprint::partition_by_fingerprint(
        &mut report.dead_code,
        prints.get(&fingerprint::Namespace::DeadCode),
        |p| noisy.contains(&p.class.to_string()),
    );
    fingerprint::partition_by_fingerprint(
        &mut report.boundaries,
        prints.get(&fingerprint::Namespace::Boundaries),
        |p| noisy.contains(&p.class.to_string()),
    );
    fingerprint::partition_by_fingerprint(
        &mut report.duplication,
        prints.get(&fingerprint::Namespace::Duplication),
        |p| noisy.contains(&p.class.to_string()),
    );
    fingerprint::partition_by_fingerprint(
        &mut report.smells,
        prints.get(&fingerprint::Namespace::Smells),
        |p| noisy.contains(&p.class.to_string()),
    );
}

fn noisy_detectors(root: &Path) -> BTreeSet<String> {
    hub::cached_noisy(root, || {
        report::low_precision_detectors(&store::load_totals(root))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brainz::fingerprint::Print;

    #[test]
    fn partition_by_fingerprint_sinks_matching_items_keeping_order() {
        let print = |hash, noisy| {
            Print::identity(
                hash,
                fingerprint::Namespace::Smells,
                fingerprint::Label::Smell {
                    smell: crate::report::SmellKind::LongFunction,
                    file: String::new(),
                    symbol: String::new(),
                },
                fingerprint::Detector::Smell {
                    smell: if noisy {
                        crate::report::SmellKind::GodModule
                    } else {
                        crate::report::SmellKind::LongFunction
                    },
                },
            )
        };
        let mut items = vec!["a", "b", "c", "d"];
        let prints = vec![
            print(1, false),
            print(2, true),
            print(3, false),
            print(4, true),
        ];
        fingerprint::partition_by_fingerprint(
            &mut items,
            Some(&prints),
            |p| matches!(p.class, fingerprint::Detector::Smell { smell } if smell == crate::report::SmellKind::GodModule),
        );
        assert_eq!(items, vec!["a", "c", "b", "d"]);
    }
}

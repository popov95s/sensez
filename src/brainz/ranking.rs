//! Precision-aware presentation ranking for findings.

use super::fingerprint::{self, Print};
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
    demote_noisy(
        &mut report.cycles,
        prints.get(&fingerprint::Namespace::Cycles),
        &noisy,
    );
    demote_noisy(
        &mut report.dead_code,
        prints.get(&fingerprint::Namespace::DeadCode),
        &noisy,
    );
    demote_noisy(
        &mut report.boundaries,
        prints.get(&fingerprint::Namespace::Boundaries),
        &noisy,
    );
    demote_noisy(
        &mut report.duplication,
        prints.get(&fingerprint::Namespace::Duplication),
        &noisy,
    );
    demote_noisy(
        &mut report.smells,
        prints.get(&fingerprint::Namespace::Smells),
        &noisy,
    );
}

fn noisy_detectors(root: &Path) -> BTreeSet<String> {
    hub::cached_noisy(root, || {
        report::low_precision_detectors(&store::load_totals(root))
    })
}

fn demote_noisy<T>(items: &mut Vec<T>, prints: Option<&Vec<Print>>, noisy: &BTreeSet<String>) {
    let Some(prints) = prints else {
        return;
    };
    let (mut trusted, mut sink) = (Vec::new(), Vec::new());
    for (i, item) in std::mem::take(items).into_iter().enumerate() {
        let is_noisy = prints
            .get(i)
            .map(|p| noisy.contains(&p.class.to_string()))
            .unwrap_or(false);
        if is_noisy {
            sink.push(item);
        } else {
            trusted.push(item);
        }
    }
    trusted.append(&mut sink);
    *items = trusted;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demote_noisy_sinks_low_precision_keeping_order() {
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
        let noisy: BTreeSet<String> = ["smells/god_module".to_string()].into();
        demote_noisy(&mut items, Some(&prints), &noisy);
        assert_eq!(items, vec!["a", "c", "b", "d"]);
    }
}

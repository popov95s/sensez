//! Precision-aware presentation ranking for findings.

use super::{hub, report, resolve, store};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn regressions(root: &Path, report: &Value) -> Vec<String> {
    let history = store::load_resolved_history(root, &hub::branch_key(root));
    if history.is_empty() {
        return Vec::new();
    }
    resolve::fingerprints(report)
        .values()
        .flatten()
        .filter(|p| history.contains_key(&format!("{:x}", p.hash)))
        .map(|p| p.label.clone())
        .collect()
}

pub fn rank_by_precision(root: &Path, report: &mut crate::noze::AnalysisReport) {
    let noisy = noisy_detectors(root);
    if noisy.is_empty() {
        return;
    }
    let Ok(value) = serde_json::to_value(&*report) else {
        return;
    };
    let prints = resolve::fingerprints(&value);
    demote_noisy(&mut report.cycles, prints.get("cycles"), &noisy);
    demote_noisy(&mut report.dead_code, prints.get("dead_code"), &noisy);
    demote_noisy(&mut report.boundaries, prints.get("boundaries"), &noisy);
    demote_noisy(&mut report.duplication, prints.get("duplication"), &noisy);
    demote_noisy(&mut report.smells, prints.get("smells"), &noisy);
}

fn noisy_detectors(root: &Path) -> BTreeSet<String> {
    hub::cached_noisy(root, || {
        report::low_precision_detectors(&store::load_totals(root))
    })
}

fn demote_noisy<T>(
    items: &mut Vec<T>,
    prints: Option<&Vec<resolve::Print>>,
    noisy: &BTreeSet<String>,
) {
    let Some(prints) = prints else {
        return;
    };
    let (mut trusted, mut sink) = (Vec::new(), Vec::new());
    for (i, item) in std::mem::take(items).into_iter().enumerate() {
        let is_noisy = prints
            .get(i)
            .map(|p| noisy.contains(&p.detector))
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
        use resolve::Print;
        let print = |hash, detector: &str| Print {
            hash,
            label: String::new(),
            detector: detector.into(),
        };
        let mut items = vec!["a", "b", "c", "d"];
        let prints = vec![
            print(1, "smells/good"),
            print(2, "smells/noisy"),
            print(3, "smells/good"),
            print(4, "smells/noisy"),
        ];
        let noisy: BTreeSet<String> = ["smells/noisy".to_string()].into();
        demote_noisy(&mut items, Some(&prints), &noisy);
        assert_eq!(items, vec!["a", "c", "b", "d"]);
    }
}

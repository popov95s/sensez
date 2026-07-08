//! Stable finding fingerprints.
//!
//! A fingerprint is built only from a finding's *identity* (symbols, modules,
//! rules, participating files) — never line numbers or measured metrics — so
//! unrelated edits that shift code around don't masquerade as fixes. Each
//! fingerprint carries a human-readable label so stale findings can be shown
//! to (and triaged by) the user.

mod types;

use crate::fingerprints::{self, Fingerprint};
use crate::report::SmellKind;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub use types::{Detector, Label, Namespace};

/// One scan's findings: pillar → fingerprints in report order.
pub type Prints = fingerprints::Groups<Namespace, Label, Detector>;

/// A single finding's stable identity for a scan: a content hash, a
/// human-readable label, and the detector that produced it (`pillar/<kind>`,
/// or just the pillar for detectors without sub-kinds).
pub type Print = Fingerprint<Namespace, Label, Detector>;

/// A fingerprint persisted across scans with its age, label, and detector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgedEntry {
    pub first_seen: u64,
    pub label: Label,
    pub class: Detector,
}

/// Fingerprints banked as resolved, kept so a finding that later comes back is
/// detected as a *reintroduction* (a fix that did not stick). Key = fingerprint
/// hex. Pruned by age so the set cannot grow without bound.
pub type ResolvedHistory = BTreeMap<String, ResolvedRecord>;

/// One previously-resolved finding: its detector (to attribute the reintro) and
/// when it was banked as resolved (to expire stale history).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRecord {
    pub class: Detector,
    pub resolved_ts: u64,
}

/// Pillar → fingerprint (hex string; JSON keys must be strings) → entry.
/// Persisted as `last-scan.json`.
pub type Aged = BTreeMap<Namespace, BTreeMap<String, AgedEntry>>;

/// Fingerprint every pillar of a JSON-serialized `AnalysisReport`.
pub fn fingerprints(report: &Value) -> Prints {
    PILLARS
        .iter()
        .map(|(namespace, json_key, key_fn)| {
            let prints = report
                .get(*json_key)
                .and_then(Value::as_array)
                .map(|findings| {
                    findings
                        .iter()
                        .map(|finding| {
                            let key = key_fn(finding);
                            Print::identity(key.hash, key.namespace, key.label, key.detector)
                        })
                        .collect()
                })
                .unwrap_or_default();
            (*namespace, prints)
        })
        .collect()
}

/// Retain items whose fingerprint satisfies the predicate.
///
/// Walks `items` in lockstep with the corresponding `prints` vector (which must
/// be in the same order). Items without a matching print are kept by default.
pub fn retain_by_fingerprint<T>(
    items: &mut Vec<T>,
    prints: Option<&Vec<Print>>,
    predicate: impl Fn(&Print) -> bool,
) {
    let Some(prints) = prints else {
        return;
    };
    let mut i = 0;
    items.retain(|_| {
        let keep = prints.get(i).map(&predicate).unwrap_or(true);
        i += 1;
        keep
    });
}

/// Partition items by fingerprint predicate, moving matching items to the end.
///
/// Walks `items` in lockstep with the corresponding `prints` vector (which must
/// be in the same order). Items whose fingerprint satisfies the predicate are
/// moved to the end while preserving relative order within each group.
pub fn partition_by_fingerprint<T>(
    items: &mut Vec<T>,
    prints: Option<&Vec<Print>>,
    predicate: impl Fn(&Print) -> bool,
) {
    let Some(prints) = prints else {
        return;
    };
    let (mut trusted, mut sink) = (Vec::new(), Vec::new());
    for (i, item) in std::mem::take(items).into_iter().enumerate() {
        let matches = prints.get(i).map(&predicate).unwrap_or(false);
        if matches {
            sink.push(item);
        } else {
            trusted.push(item);
        }
    }
    trusted.append(&mut sink);
    *items = trusted;
}

/// Per-detector reported counts for a report (e.g. `smells/god_module → 4`).
/// Recorded on every scan, including diff/gate scans that skip aging.
pub fn detector_counts(report: &Value) -> BTreeMap<String, u64> {
    fingerprints::class_counts(&fingerprints(report))
}

/// Stable identity fields for one finding.
struct FingerprintKey {
    hash: u64,
    namespace: Namespace,
    label: Label,
    detector: Detector,
}

type KeyFn = fn(&Value) -> FingerprintKey;

const PILLARS: [(Namespace, &str, KeyFn); 5] = [
    (Namespace::Cycles, "cycles", cycle_key),
    (Namespace::DeadCode, "dead_code", dead_code_key),
    (Namespace::Boundaries, "boundaries", boundary_key),
    (Namespace::Duplication, "duplication", clone_key),
    (Namespace::Smells, "smells", smell_key),
];

fn hash_parts(parts: &[&str]) -> u64 {
    fingerprints::hash_parts(parts)
}

fn field<'v>(finding: &'v Value, key: &str) -> &'v str {
    finding.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// Identity: the set of modules in the cycle (order-independent).
fn cycle_key(finding: &Value) -> FingerprintKey {
    let mut modules: Vec<&str> = finding
        .get("modules")
        .and_then(Value::as_array)
        .map(|m| m.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    modules.sort_unstable();
    let label = modules.join(" ↔ ");
    FingerprintKey {
        hash: hash_parts(&modules),
        label: Label::Cycle { modules: label },
        namespace: Namespace::Cycles,
        detector: Detector::Cycles,
    }
}

/// Identity: which symbol in which module (confidence may legitimately change).
/// Detector sub-kind is the symbol kind (function / class / method / import).
fn dead_code_key(finding: &Value) -> FingerprintKey {
    let (module, symbol, kind) = (
        field(finding, "module"),
        field(finding, "symbol"),
        field(finding, "kind"),
    );
    FingerprintKey {
        hash: hash_parts(&[module, symbol, kind]),
        label: Label::DeadCode {
            module: module.to_string(),
            symbol: symbol.to_string(),
            symbol_kind: kind.to_string(),
        },
        namespace: Namespace::DeadCode,
        detector: Detector::DeadCode {
            symbol_kind: kind.to_string(),
        },
    }
}

/// Identity: the forbidden edge and the rule that forbids it.
fn boundary_key(finding: &Value) -> FingerprintKey {
    let (from, to, rule) = (
        field(finding, "from_module"),
        field(finding, "to_module"),
        field(finding, "rule"),
    );
    FingerprintKey {
        hash: hash_parts(&[from, to, rule]),
        label: Label::Boundary {
            from: from.to_string(),
            to: to.to_string(),
            rule: rule.to_string(),
        },
        namespace: Namespace::Boundaries,
        detector: Detector::Boundaries,
    }
}

/// Identity: the sorted set of files the clone class spans plus its arity.
/// Rows shift on every edit, so they are deliberately excluded.
fn clone_key(finding: &Value) -> FingerprintKey {
    let mut files: Vec<&str> = finding
        .get("occurrences")
        .and_then(Value::as_array)
        .map(|occ| occ.iter().map(|o| field(o, "file")).collect())
        .unwrap_or_default();
    let arity = files.len().to_string();
    files.sort_unstable();
    files.dedup();
    let label = files.join(" + ");
    files.push(&arity);
    FingerprintKey {
        hash: hash_parts(&files),
        label: Label::Clone {
            arity: arity.parse().unwrap_or_default(),
            files: label,
        },
        namespace: Namespace::Duplication,
        detector: Detector::Duplication,
    }
}

/// Identity: smell kind on a symbol in a file (metric value may fluctuate).
/// Detector sub-kind is the smell kind (long_function / god_module / …).
fn smell_key(finding: &Value) -> FingerprintKey {
    let (kind, file, symbol) = (
        field(finding, "kind"),
        field(finding, "file"),
        field(finding, "symbol"),
    );
    let smell = serde_json::from_value(finding.get("kind").cloned().unwrap_or(Value::Null))
        .unwrap_or(SmellKind::LongFunction);
    FingerprintKey {
        hash: hash_parts(&[kind, file, symbol]),
        label: Label::Smell {
            smell,
            file: file.to_string(),
            symbol: symbol.to_string(),
        },
        namespace: Namespace::Smells,
        detector: Detector::Smell { smell },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cycle_identity_ignores_module_order() {
        let a = cycle_key(&json!({"modules": ["x", "y"]}));
        let b = cycle_key(&json!({"modules": ["y", "x"]}));
        assert_eq!(a.hash, b.hash);
        assert_eq!(a.detector, Detector::Cycles);
    }
}

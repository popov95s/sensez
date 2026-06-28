//! Stable finding fingerprints.
//!
//! A fingerprint is built only from a finding's *identity* (symbols, modules,
//! rules, participating files) — never line numbers or measured metrics — so
//! unrelated edits that shift code around don't masquerade as fixes. Each
//! fingerprint carries a human-readable label so stale findings can be shown
//! to (and triaged by) the user.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

/// One scan's findings: pillar → fingerprints in report order.
pub type Prints = BTreeMap<String, Vec<Print>>;

/// A single finding's stable identity for a scan: a content hash, a
/// human-readable label, and the detector that produced it (`pillar/<kind>`,
/// or just the pillar for detectors without sub-kinds).
#[derive(Debug, Clone)]
pub struct Print {
    pub hash: u64,
    pub label: String,
    pub detector: String,
}

/// A fingerprint persisted across scans with its age, label, and detector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgedEntry {
    pub first_seen: u64,
    pub label: String,
    /// Detector id (`pillar/<kind>`, or just the pillar).
    pub detector: String,
}

/// Fingerprints banked as resolved, kept so a finding that later comes back is
/// detected as a *reintroduction* (a fix that did not stick). Key = fingerprint
/// hex. Pruned by age so the set cannot grow without bound.
pub type ResolvedHistory = BTreeMap<String, ResolvedRecord>;

/// One previously-resolved finding: its detector (to attribute the reintro) and
/// when it was banked as resolved (to expire stale history).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRecord {
    pub detector: String,
    pub resolved_ts: u64,
}

/// Pillar → fingerprint (hex string; JSON keys must be strings) → entry.
/// Persisted as `last-scan.json`.
pub type Aged = BTreeMap<String, BTreeMap<String, AgedEntry>>;

/// Fingerprint every pillar of a JSON-serialized `AnalysisReport`.
pub fn fingerprints(report: &Value) -> Prints {
    PILLARS
        .iter()
        .map(|(pillar, key_fn)| {
            let prints = report
                .get(*pillar)
                .and_then(Value::as_array)
                .map(|findings| {
                    findings
                        .iter()
                        .map(|finding| {
                            let key = key_fn(finding);
                            let detector = match key.sub_kind {
                                Some(kind) if !kind.is_empty() => format!("{pillar}/{kind}"),
                                _ => pillar.to_string(),
                            };
                            Print {
                                hash: key.hash,
                                label: key.label,
                                detector,
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            (pillar.to_string(), prints)
        })
        .collect()
}

/// Per-detector reported counts for a report (e.g. `smells/god_module → 4`).
/// Recorded on every scan, including diff/gate scans that skip aging.
pub fn detector_counts(report: &Value) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for prints in fingerprints(report).values() {
        for print in prints {
            *counts.entry(print.detector.clone()).or_default() += 1;
        }
    }
    counts
}

/// Stable identity fields for one finding. `None` sub-kind means the pillar has
/// no finer detector (the detector id is then just the pillar).
struct FingerprintKey {
    hash: u64,
    label: String,
    sub_kind: Option<String>,
}

type KeyFn = fn(&Value) -> FingerprintKey;

const PILLARS: [(&str, KeyFn); 5] = [
    ("cycles", cycle_key),
    ("dead_code", dead_code_key),
    ("boundaries", boundary_key),
    ("duplication", clone_key),
    ("smells", smell_key),
];

fn hash_parts(parts: &[&str]) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    for part in parts {
        part.hash(&mut hasher);
    }
    hasher.finish()
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
    FingerprintKey {
        hash: hash_parts(&modules),
        label: modules.join(" ↔ "),
        sub_kind: None,
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
        label: format!("{module}::{symbol} ({kind})"),
        sub_kind: Some(kind.to_string()),
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
        label: format!("{from} → {to} ({rule})"),
        sub_kind: None,
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
    let label = format!("clone x{arity}: {}", files.join(" + "));
    files.push(&arity);
    FingerprintKey {
        hash: hash_parts(&files),
        label,
        sub_kind: None,
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
    FingerprintKey {
        hash: hash_parts(&[kind, file, symbol]),
        label: format!("{kind} @ {file}::{symbol}"),
        sub_kind: Some(kind.to_string()),
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
        assert_eq!(a.sub_kind, None, "cycles have no detector sub-kind");
    }
}

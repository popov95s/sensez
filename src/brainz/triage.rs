//! Human triage of stale findings: persistent "debt" vs "false_positive"
//! verdicts, recorded via the `brainz_triage` MCP tool on the user's behalf.
//! Verdicts live in `.sensez/local-metrics/triage.json`, keyed by finding fingerprint,
//! and feed back into the metrics: triaged findings stop appearing as stale,
//! and a vanished false positive is never counted as resolved value.

use super::file_lock;
use super::fingerprint::{Detector, Pillar};
use super::store;
use super::verdict::TriageVerdict;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One human verdict on a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    /// `Debt` (real, deliberately deferred) or `FalsePositive` (wrong).
    pub verdict: TriageVerdict,
    pub pillar: Pillar,
    /// Label at triage time (kept so reports stay readable after the finding
    /// leaves the scan output).
    pub label: String,
    pub ts: u64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub note: Option<String>,
}

/// One finding a triage call marked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marked {
    pub label: String,
    pub detector: Detector,
}

/// Fingerprint key (hex) → verdict.
pub type Triage = BTreeMap<String, Verdict>;

pub fn load(root: &Path) -> Triage {
    super::json_file::load_or_default(&store::dir(root).join("triage.json"))
}

fn save(root: &Path, triage: &Triage) -> Result<()> {
    let d = crate::dotdir::ensure(root, Some("local-metrics"))?;
    let json = serde_json::to_vec_pretty(triage).context("serializing triage")?;
    store::write_durable(&d.join("triage.json"), &json)
}

/// Mark every finding in `pillar` whose label contains `pattern`
/// (case-insensitive) with `verdict`; `Clear` removes existing marks instead.
/// Errors when nothing matches so the agent can show what is available.
pub fn mark(
    root: &Path,
    branch: &str,
    pillar: Pillar,
    pattern: &str,
    verdict: TriageVerdict,
    note: Option<String>,
) -> Result<Vec<Marked>> {
    let aged = store::load_fingerprints(root, branch);
    let needle = pattern.to_lowercase();
    let matches: Vec<(String, Marked)> = aged
        .get(&pillar)
        .map(|prints| {
            prints
                .iter()
                .filter(|(_, e)| e.label.to_string().to_lowercase().contains(&needle))
                .map(|(key, e)| {
                    let marked = Marked {
                        label: e.label.to_string(),
                        detector: e.class.clone(),
                    };
                    (key.clone(), marked)
                })
                .collect()
        })
        .unwrap_or_default();
    if matches.is_empty() {
        let known: Vec<String> = aged
            .get(&pillar)
            .map(|p| p.values().map(|e| e.label.to_string()).collect())
            .unwrap_or_default();
        return Err(anyhow!(
            "no '{pillar}' finding matches '{pattern}'; known: {}",
            if known.is_empty() {
                "none (scan first)".to_string()
            } else {
                known.join(" | ")
            }
        ));
    }

    let _lock = file_lock::acquire(root, "triage.lock")?;
    let mut triage = load(root);
    for (key, marked) in &matches {
        if verdict == TriageVerdict::Clear {
            triage.remove(key);
        } else {
            triage.insert(
                key.clone(),
                Verdict {
                    verdict,
                    pillar,
                    label: marked.label.clone(),
                    ts: super::hub::now(),
                    note: note.clone(),
                },
            );
        }
    }
    save(root, &triage)?;
    Ok(matches.into_iter().map(|(_, marked)| marked).collect())
}

/// Fingerprint keys excluded from stale/resolved accounting (all triaged).
pub fn ignored_keys(triage: &Triage) -> std::collections::HashSet<String> {
    triage.keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brainz::fingerprint::{Aged, AgedEntry, Label};
    use std::fs;

    #[test]
    fn mark_matches_by_label_and_clears() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        fs::create_dir_all(&root).unwrap();

        let aged: Aged = BTreeMap::from([(
            Pillar::DeadCode,
            BTreeMap::from([(
                "abc1".to_string(),
                AgedEntry {
                    first_seen: 1,
                    label: Label::DeadCode {
                        module: "app".to_string(),
                        symbol: "orphan".to_string(),
                        symbol_kind: "function".to_string(),
                    },
                    class: Detector::DeadCode {
                        symbol_kind: "function".to_string(),
                    },
                },
            )]),
        )]);
        store::save_fingerprints(&root, "main", &aged, &Default::default(), 1).unwrap();

        assert!(mark(
            &root,
            "main",
            Pillar::DeadCode,
            "nomatch",
            TriageVerdict::Debt,
            None
        )
        .is_err());

        let hit = mark(
            &root,
            "main",
            Pillar::DeadCode,
            "ORPHAN",
            TriageVerdict::FalsePositive,
            Some("dynamic dispatch".into()),
        )
        .unwrap();
        assert_eq!(
            hit,
            vec![Marked {
                label: "app::orphan (function)".to_string(),
                detector: Detector::DeadCode {
                    symbol_kind: "function".to_string(),
                },
            }]
        );
        let triage = load(&root);
        assert_eq!(triage["abc1"].verdict, TriageVerdict::FalsePositive);
        assert!(ignored_keys(&triage).contains("abc1"));

        mark(
            &root,
            "main",
            Pillar::DeadCode,
            "orphan",
            TriageVerdict::Clear,
            None,
        )
        .unwrap();
        assert!(load(&root).is_empty());
    }
}

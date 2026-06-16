//! Human triage of stale findings: persistent "debt" vs "false_positive"
//! verdicts, recorded via the `brainz_triage` MCP tool on the user's behalf.
//! Verdicts live in `.sensez/local-metrics/triage.json`, keyed by finding fingerprint,
//! and feed back into the metrics: triaged findings stop appearing as stale,
//! and a vanished false positive is never counted as resolved value.

use super::store;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// One human verdict on a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    /// "debt" (real, deliberately deferred) or "false_positive" (wrong).
    pub verdict: String,
    pub pillar: String,
    /// Label at triage time (kept so reports stay readable after the finding
    /// leaves the scan output).
    pub label: String,
    pub ts: u64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub note: Option<String>,
}

/// Fingerprint key (hex) → verdict.
pub type Triage = BTreeMap<String, Verdict>;

pub fn load(root: &Path) -> Triage {
    fs::read(store::dir(root).join("triage.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save(root: &Path, triage: &Triage) -> Result<()> {
    let d = crate::dotdir::ensure(root, Some("local-metrics"))?;
    let json = serde_json::to_vec_pretty(triage).context("serializing triage")?;
    fs::write(d.join("triage.json"), json).context("writing triage.json")?;
    Ok(())
}

/// Mark every finding in `pillar` whose label contains `pattern`
/// (case-insensitive) with `verdict`. `verdict == "clear"` removes existing
/// marks instead. Returns `(label, detector)` for each affected finding;
/// errors when nothing matches so the agent can show what is available.
pub fn mark(
    root: &Path,
    branch: &str,
    pillar: &str,
    pattern: &str,
    verdict: &str,
    note: Option<String>,
) -> Result<Vec<(String, String)>> {
    if !matches!(verdict, "debt" | "false_positive" | "clear") {
        return Err(anyhow!("verdict must be debt | false_positive | clear"));
    }
    let aged = store::load_fingerprints(root, branch);
    let needle = pattern.to_lowercase();
    // (fingerprint key, label, detector) for each matched finding.
    let matches: Vec<(String, String, String)> = aged
        .get(pillar)
        .map(|prints| {
            prints
                .iter()
                .filter(|(_, e)| e.label.to_lowercase().contains(&needle))
                .map(|(k, e)| (k.clone(), e.label.clone(), e.detector.clone()))
                .collect()
        })
        .unwrap_or_default();
    if matches.is_empty() {
        let known: Vec<String> = aged
            .get(pillar)
            .map(|p| p.values().map(|e| e.label.clone()).collect())
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

    let mut triage = load(root);
    let out: Vec<(String, String)> = matches
        .iter()
        .map(|(_, label, detector)| (label.clone(), detector.clone()))
        .collect();
    for (key, label, _) in matches {
        if verdict == "clear" {
            triage.remove(&key);
        } else {
            triage.insert(
                key,
                Verdict {
                    verdict: verdict.to_string(),
                    pillar: pillar.to_string(),
                    label,
                    ts: super::hub::now(),
                    note: note.clone(),
                },
            );
        }
    }
    save(root, &triage)?;
    Ok(out)
}

/// Fingerprint keys excluded from stale/resolved accounting (all triaged).
pub fn ignored_keys(triage: &Triage) -> std::collections::HashSet<String> {
    triage.keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brainz::resolve::{Aged, AgedEntry};

    #[test]
    fn mark_matches_by_label_and_clears() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        fs::create_dir_all(&root).unwrap();

        let aged: Aged = BTreeMap::from([(
            "dead_code".to_string(),
            BTreeMap::from([(
                "abc1".to_string(),
                AgedEntry {
                    first_seen: 1,
                    label: "app::orphan (function)".to_string(),
                    detector: "dead_code/function".to_string(),
                },
            )]),
        )]);
        store::save_fingerprints(&root, "main", &aged, &Default::default(), 1).unwrap();

        assert!(mark(&root, "main", "dead_code", "nomatch", "debt", None).is_err());
        assert!(mark(&root, "main", "dead_code", "orphan", "bogus", None).is_err());

        let hit = mark(
            &root,
            "main",
            "dead_code",
            "ORPHAN",
            "false_positive",
            Some("dynamic dispatch".into()),
        )
        .unwrap();
        assert_eq!(
            hit,
            vec![(
                "app::orphan (function)".to_string(),
                "dead_code/function".to_string()
            )]
        );
        let triage = load(&root);
        assert_eq!(triage["abc1"].verdict, "false_positive");
        assert!(ignored_keys(&triage).contains("abc1"));

        mark(&root, "main", "dead_code", "orphan", "clear", None).unwrap();
        assert!(load(&root).is_empty());
    }
}

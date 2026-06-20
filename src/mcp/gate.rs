use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn last_gated() -> &'static Mutex<HashMap<PathBuf, u64>> {
    static MAP: OnceLock<Mutex<HashMap<PathBuf, u64>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn gate(args: &Value) -> super::handlers::ToolResult {
    let path = super::handlers::required_str(args, "path")?;
    if hook_already_blocked(args) {
        return Ok(allow());
    }
    let root = Path::new(path);
    let Ok(changed) = crate::diff::git::changed_vs_head(root) else {
        return Ok(allow());
    };
    if changed.is_empty() {
        return Ok(allow());
    }
    let sig = working_signature(&changed);
    {
        let mut map = last_gated().lock().unwrap_or_else(|e| e.into_inner());
        if map.get(root) == Some(&sig) {
            return Ok(allow());
        }
        map.insert(root.to_path_buf(), sig);
    }

    let start = std::time::Instant::now();
    let gate_config = crate::config::model::Config::load(root)
        .map(|config| config.gate)
        .unwrap_or_default();
    let Ok(mut report) = crate::analyze_path(root, None, Some(&changed)) else {
        return Ok(allow());
    };
    crate::brainz::apply_suppressions(root, &mut report);
    let repeats = super::repeats::suppress_repeated(root, &mut report, gate_config.repeat_limit);
    crate::brainz::rank_by_precision(root, &mut report);
    let full = serde_json::to_value(&report).unwrap_or(Value::Null);
    crate::brainz::record_scan(
        root,
        &full,
        crate::brainz::BaselineUpdate::Preserve,
        start.elapsed().as_millis() as u64,
        None,
        crate::brainz::Origin::Gate,
    );

    let n = report.duplication.len()
        + report.dead_code.len()
        + report.cycles.len()
        + report.boundaries.len()
        + report.smells.len();
    if n == 0 {
        return Ok(allow());
    }
    crate::brainz::record_gate_block(root, &full);
    let regressed = crate::brainz::regressions(root, &full);
    let escalation = if regressed.is_empty() {
        String::new()
    } else {
        format!(
            " ⚠ REGRESSION: {} finding(s) you are reintroducing were previously \
             fixed — prioritize these: {}.",
            regressed.len(),
            regressed.join("; ")
        )
    };

    let mut top = report;
    crate::noze::limit(&mut top, 5);
    let findings = crate::reporter::to_json(&top).unwrap_or_else(|_| "{}".to_string());
    let deferred_note = if repeats.deferred == 0 {
        String::new()
    } else {
        format!(
            " Auto-deferred {} finding(s) already reported at least {} time(s) on the same line(s).",
            repeats.deferred,
            gate_config.repeat_limit
        )
    };
    let decision = json!({
        "decision": "block",
        "reason": format!(
            "sensez diff-scan: {n} finding(s) touch code this session changed. Fix \
             the ones your change introduced or worsened. A finding in \
             pre-existing code you are deliberately not addressing is accepted \
             DEBT, not a false positive — just say so briefly to the user and \
             proceed; do not call any tool to report it (Sensez records \
             automatically). You will be allowed to finish after this one pass.{deferred_note}{escalation} \
             Findings (top 5 per pillar): {findings}"
        ),
    });
    Ok(super::handlers::text_result(decision.to_string(), false))
}

fn working_signature(changed: &crate::diff::ChangedLines) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut paths: Vec<&Path> = changed.paths().collect();
    paths.sort_unstable();
    let mut h = rustc_hash::FxHasher::default();
    for p in paths {
        p.hash(&mut h);
        if let Ok(meta) = std::fs::metadata(p) {
            meta.len().hash(&mut h);
            if let Ok(modified) = meta.modified() {
                if let Ok(d) = modified.duration_since(std::time::UNIX_EPOCH) {
                    d.as_nanos().hash(&mut h);
                }
            }
        }
    }
    h.finish()
}

fn hook_already_blocked(args: &Value) -> bool {
    match args.get("stop_hook_active") {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "true",
        _ => false,
    }
}

fn allow() -> Value {
    super::handlers::text_result("{}".to_string(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_degrades_open() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().into_owned();

        let resp = gate(&json!({"path": path, "stop_hook_active": false})).unwrap();
        assert_eq!(resp["content"][0]["text"], "{}", "non-git repo -> allow");

        let resp = gate(&json!({"path": path, "stop_hook_active": "true"})).unwrap();
        assert_eq!(resp["content"][0]["text"], "{}", "second stop -> allow");
    }

    #[test]
    fn signature_tracks_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("a.py");
        std::fs::write(&file, "x = 1\n").unwrap();
        let mut changed = crate::diff::ChangedLines::default();
        changed.add_full_file(&file);

        let sig1 = working_signature(&changed);
        assert_eq!(sig1, working_signature(&changed), "stable when untouched");

        std::fs::write(&file, "x = 1\ny = 2\nz = 3\n").unwrap();
        assert_ne!(sig1, working_signature(&changed), "changes after a write");
    }
}

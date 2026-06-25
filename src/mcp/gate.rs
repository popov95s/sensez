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
    let sig = changed.signature();
    {
        let mut map = last_gated().lock().unwrap_or_else(|e| e.into_inner());
        if map.get(root) == Some(&sig) {
            return Ok(allow());
        }
        map.insert(root.to_path_buf(), sig);
    }

    let gate_config = crate::config::model::Config::load(root)
        .map(|config| config.gate)
        .unwrap_or_default();
    let Ok((mut report, _snapshot)) =
        super::scan::run_and_record(root, None, 0, true, crate::brainz::Origin::Gate)
    else {
        return Ok(allow());
    };

    // The auto-defer UX is gate-specific; it sees the post-processed
    // report (already ranked + issues-cleared) and drops any finding
    // that has been reported on the same lines too many times.
    let repeats = super::repeats::suppress_repeated(root, &mut report, gate_config.repeat_limit);

    let n = report.duplication.len()
        + report.dead_code.len()
        + report.cycles.len()
        + report.boundaries.len()
        + report.smells.len();
    if n == 0 {
        return Ok(allow());
    }

    let diff_report = serde_json::to_value(&report).unwrap_or(Value::Null);
    crate::brainz::record_gate_block(root, &diff_report);
    let regressed = crate::brainz::regressions(root, &diff_report);
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

    let findings = report.top_n_summary(5);
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
            "sensez gate: {n} diff finding(s) need attention. Fix findings your \
             change introduced or worsened; briefly note intentional debt and \
             continue. The gate will not keep blocking the same unchanged work.{deferred_note}{escalation} \
             Top findings: {findings}"
        ),
    });
    Ok(super::handlers::text_result(decision.to_string(), false))
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

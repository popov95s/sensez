use serde_json::{json, Value};
use std::path::Path;

pub(super) fn gate(args: &Value) -> super::handlers::ToolResult {
    let path = super::handlers::required_str(args, "path")?;
    let root = Path::new(path);
    let changed = match crate::diff::git::changed_vs_head(root) {
        Ok(changed) => changed,
        Err(err) => {
            eprintln!("[sensez gate] failed to get git diff, allowing: {err:#}");
            return Ok(allow());
        }
    };
    if changed.is_empty() {
        return Ok(allow());
    }

    let gate_config = crate::config::model::Config::load(root)
        .map(|config| config.gate)
        .unwrap_or_default();
    let (mut report, snapshot, elapsed) = match super::scan::diff(root, None, 0) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("[sensez gate] failed to scan, allowing: {err:#}");
            return Ok(allow());
        }
    };
    crate::brainz::record_scan(root, &snapshot, elapsed, None, crate::brainz::Origin::Gate);

    // The auto-defer UX is gate-specific; it sees the post-processed
    // report (already ranked + issues-cleared) and drops any finding
    // that has been reported on the same lines too many times.
    let repeats = super::repeats::suppress_repeated(root, &mut report, gate_config.repeat_limit);

    let n = crate::brainz::retain_unseen_gate_findings(root, &mut report);
    if n == 0 {
        return Ok(allow());
    }

    let diff_report = serde_json::to_value(&report).unwrap_or(Value::Null);
    crate::brainz::record_gate_block(root, &diff_report);
    crate::brainz::flush();
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

fn allow() -> Value {
    super::handlers::text_result("{}".to_string(), false)
}

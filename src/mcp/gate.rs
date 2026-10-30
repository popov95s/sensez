use super::gate_state::ScopeState;
use crate::report::AnalysisReport;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn gate(args: &Value) -> super::handlers::ToolResult {
    let root = match super::gate_input::repository_path(args) {
        Ok(root) => root,
        Err(message) => return Ok(super::handlers::text_result(message, true)),
    };
    if super::gate_input::stop_hook_active(args) {
        return Ok(allow());
    }
    let root = root.as_path();
    let scope = super::agent_scope::id(super::gate_input::templated(args, "session_id"));
    let changed = match changed_for_gate(root, args) {
        Ok(changed) => changed,
        Err(err) => {
            eprintln!("[sensez gate] failed to resolve changed scope, allowing: {err:#}");
            return Ok(allow());
        }
    };
    if changed.is_empty() {
        return Ok(allow());
    }

    let gate_config = crate::config::model::Config::load(root)
        .map(|config| config.gate)
        .unwrap_or_default();
    let (mut report, snapshot, elapsed) =
        match super::scan::run(root, None, 0, super::scan::Scope::Changed(changed)) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("[sensez gate] failed to scan, allowing: {err:#}");
                return Ok(allow());
            }
        };
    let branch = crate::diff::git::current_branch(root);
    crate::brainz::record_scan(
        root,
        branch.as_deref(),
        &snapshot,
        elapsed,
        None,
        crate::brainz::Origin::Gate,
    );
    let now = super::gate_state::now_secs();
    let outcome = super::gate_state::update(root, scope.as_deref(), now, |state| {
        triage(
            state,
            &mut report,
            branch.as_deref().unwrap_or("unbranched"),
            gate_config.repeat_limit,
            now,
        )
    });
    let n = report.finding_count();
    if n == 0 {
        return Ok(allow());
    }

    let diff_report = serde_json::to_value(&report).unwrap_or(Value::Null);
    crate::brainz::record_gate_block(root, branch.as_deref(), scope.as_deref(), &diff_report);
    crate::brainz::flush();
    let regressed = regression_labels(
        root,
        branch.as_deref(),
        &report,
        &diff_report,
        &outcome.reintroduced,
    );
    let escalation = if regressed.is_empty() {
        String::new()
    } else {
        format!(
            " ⚠ REGRESSION: {} finding(s) you are reintroducing were previously \
             fixed — prioritize these: {}.",
            regressed.len(),
            regressed.into_iter().collect::<Vec<_>>().join("; ")
        )
    };

    let findings = report.top_n_summary(5);
    let deferred_note = if outcome.deferred == 0 {
        String::new()
    } else {
        format!(
            " Auto-deferred {} finding(s) already reported at least {} time(s) on the same line(s).",
            outcome.deferred, gate_config.repeat_limit
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

struct Triage {
    reintroduced: BTreeSet<String>,
    deferred: usize,
}

/// Prune already-blocked findings, then repeats, then remember what is left as blocked.
fn triage(
    state: &mut ScopeState,
    report: &mut AnalysisReport,
    branch: &str,
    repeat_limit: usize,
    now: u64,
) -> Triage {
    let reintroduced = state.recall(report);
    let repeats = state.repeats.entry(branch.to_string()).or_default();
    let deferred = super::repeats::suppress_repeated(repeats, report, repeat_limit, now).deferred;
    state.remember(report);
    Triage {
        reintroduced,
        deferred,
    }
}

fn regression_labels(
    root: &Path,
    branch: Option<&str>,
    report: &AnalysisReport,
    diff_report: &Value,
    reintroduced: &BTreeSet<String>,
) -> BTreeSet<String> {
    let from_gate = crate::brainz::finding_identities(report)
        .into_iter()
        .filter(|(key, _)| reintroduced.contains(key))
        .map(|(_, label)| label);
    from_gate
        .chain(crate::brainz::regressions(root, branch, diff_report))
        .collect()
}

fn changed_for_gate(root: &Path, args: &Value) -> anyhow::Result<crate::diff::ChangedLines> {
    match super::gate_input::templated(args, "transcript_path") {
        Some(path) => super::agent_scope::changes(root, Path::new(path)),
        None => crate::diff::git::changed_vs_head(root),
    }
}

fn allow() -> Value {
    super::handlers::text_result("{}".to_string(), false)
}

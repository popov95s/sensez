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
    super::scan_recording::suppress_scan_issues(&mut report);
    let diff_report = serde_json::to_value(&report).unwrap_or(Value::Null);
    if let Ok(snapshot) = super::scan_recording::snapshot_for_recording(root, None, &report, true) {
        crate::brainz::record_scan(
            root,
            &snapshot,
            start.elapsed().as_millis() as u64,
            None,
            crate::brainz::Origin::Gate,
        );
    }

    let n = report.duplication.len()
        + report.dead_code.len()
        + report.cycles.len()
        + report.boundaries.len()
        + report.smells.len();
    if n == 0 {
        return Ok(allow());
    }
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

    let findings = finding_summary(&report, 5);
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

fn finding_summary(report: &crate::report::AnalysisReport, max: usize) -> String {
    let mut items = Vec::new();
    items.extend(report.dead_code.iter().map(|finding| {
        format!(
            "dead_code/{} {}::{} {}:{}",
            finding.kind,
            finding.module,
            finding.symbol,
            finding.file.display(),
            finding.line
        )
    }));
    items.extend(
        report
            .cycles
            .iter()
            .map(|finding| format!("cycle {}", finding.modules.join(" -> "))),
    );
    items.extend(report.boundaries.iter().map(|finding| {
        format!(
            "boundary {} -> {} {}:{}",
            finding.from_module,
            finding.to_module,
            finding.file.display(),
            finding.line
        )
    }));
    items.extend(report.duplication.iter().map(|finding| {
        let locations = finding
            .occurrences
            .iter()
            .map(|occ| format!("{}:{}", occ.file.display(), occ.start_row))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "duplication {} token(s) at {locations}",
            finding.token_length
        )
    }));
    items.extend(report.smells.iter().map(|finding| {
        format!(
            "smell/{} {} {}:{}",
            finding.kind,
            finding.symbol,
            finding.file.display(),
            finding.line
        )
    }));
    items.truncate(max);
    items.join("; ")
}

pub(super) fn working_signature(changed: &crate::diff::ChangedLines) -> u64 {
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

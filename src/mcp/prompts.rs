//! MCP prompts: agent-agnostic slash commands carried by the server itself.
//! Any MCP client (Claude Code, Cursor, Cline, Zed, ...) discovers these via
//! `prompts/list`; Claude Code surfaces them as `/sense:report`-style commands.

use serde_json::{json, Value};

pub fn prompts_list() -> Value {
    json!({"prompts": [{
        "name": "report",
        "description": "Summarize how Sensez helped this session (fixes auto-detected, \
                        searches served, stale findings) — useful before a commit.",
        "arguments": [{
            "name": "path",
            "description": "Absolute project root (defaults to the current workspace)",
            "required": false
        }]
    }]})
}

pub fn prompts_get(params: Option<&Value>) -> Result<Value, (i64, String)> {
    let params = params.ok_or((-32602, "missing params".to_string()))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if name != "report" {
        return Err((-32602, format!("unknown prompt: {name}")));
    }
    let path = params
        .get("arguments")
        .and_then(|a| a.get("path"))
        .and_then(Value::as_str)
        .unwrap_or("the absolute root of the current repository");
    let text = format!(
        "Report the value sensez (the codebase-intelligence MCP server) provided. All \
         metrics are recorded automatically and are local-only \
         (.sensez/local-metrics/), never exported — you do not report anything \
         routine.\n\n\
         1. Call the sensez tool `brainz_report` with path = {path}.\n\
         2. Summarize impact-first: findings fixed after sensez reported them \
         (resolved_by_detector — auto-detected, with mean_resolution_days), \
         searches served (note first_searches), est_context_bytes_saved as ~tokens \
         (bytes/4), then scans run (scans_by_origin). Call out two trust signals if \
         non-trivial: precision_by_detector (detectors with low precision are noisy) \
         and recidivism_by_detector (fixes that didn't stick — likely hotspots). \
         Keep it to a short paragraph plus a small table; if everything is zero, say \
         Sensez wasn't used — don't pad.\n\
         3. If brainz_report lists stale_findings, show them to the user and ASK \
         whether each is accepted technical debt (real, deferred) or a false \
         positive (Sensez is wrong). Do not decide for them, and remember \"not mine \
         / pre-existing\" is DEBT, not a false positive. Record only the answers \
         they give via `brainz_triage` (match = a substring of the finding \
         label, note = their rationale).\n\
         4. If brainz_report.calibration is non-empty, relay those config-hygiene \
         suggestions to the user as options (they decide — Sensez never edits its own \
         config). Mention gate_conversion if the gate has blocked (how many caught \
         findings were fixed vs. escaped).\n\
         5. If run before a commit, end with one line on what remains open."
    );
    Ok(json!({
        "description": "sensez session value report",
        "messages": [{"role": "user", "content": {"type": "text", "text": text}}]
    }))
}

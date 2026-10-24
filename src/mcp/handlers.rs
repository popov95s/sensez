//! Tool-call handlers for the MCP surface.

use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
#[cfg(feature = "eyez")]
use std::time::Instant;

pub(super) type ToolResult = Result<Value, (i64, String)>;

/// JSON-RPC error code for invalid method parameters.
pub(super) const INVALID_PARAMS: i64 = -32602;

pub(super) fn call(name: &str, args: &Value) -> ToolResult {
    match name {
        "noze_sniff" | "scan" => scan_tool(args),
        "get_configuration_summary" => configuration_summary(args),
        #[cfg(feature = "eyez")]
        "eyez_search_docs" | "search_docs" => search_docs(args),
        "noze_gate" | "gate" => super::gate::gate(args),
        "noze_explain" | "explain" => explain(args),
        "brainz_triage" | "triage_finding" => triage_finding(args),
        "brainz_report" | "usage_report" => usage_report(args),
        other => Err((INVALID_PARAMS, format!("unknown tool: {other}"))),
    }
}

pub(super) fn required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, (i64, String)> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| (INVALID_PARAMS, format!("missing '{key}' argument")))
}

pub(super) fn text_result(text: String, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

fn configuration_summary(args: &Value) -> ToolResult {
    let path = Path::new(required_str(args, "path")?);
    let scope = if args.get("diff").and_then(Value::as_bool).unwrap_or(false) {
        super::scan::Scope::DiffVsHead
    } else {
        super::scan::Scope::Full
    };
    let summary =
        super::scan::run(path, None, 0, scope).and_then(|(report, _snapshot, _elapsed)| {
            let (config, _issues) = crate::config::model::Config::load_for_scan(path);
            crate::config_summary::render(path, &report, &config)
        });
    match summary {
        Ok(text) => Ok(text_result(text, false)),
        Err(err) => Ok(text_result(format!("{err:#}"), true)),
    }
}

fn scan_tool(args: &Value) -> ToolResult {
    let scan_args: ScanArgs = serde_json::from_value(args.clone())
        .map_err(|e| (INVALID_PARAMS, format!("invalid arguments: {e}")))?;

    match run_scan(Path::new(&scan_args.path), &scan_args) {
        Ok((text, _snapshot)) => {
            let mut content = vec![json!({"type": "text", "text": text})];
            if let Some(warning) = super::tools::scope_warning(Path::new(&scan_args.path)) {
                content.insert(0, json!({"type": "text", "text": warning}));
            }
            Ok(json!({"content": content, "isError": false}))
        }
        Err(err) => Ok(text_result(format!("{err:#}"), true)),
    }
}

#[derive(Debug, Deserialize)]
struct ScanArgs {
    /// Repository path to scan.
    path: String,
    /// Override the duplication token threshold for this scan.
    #[serde(default)]
    threshold: Option<usize>,
    /// Per-pillar cap on returned findings (`0` = no cap).
    #[serde(default)]
    limit: usize,
    /// Scope the scan to the diff vs. HEAD (`true` is the agent-friendly
    /// default; pass `false` for a full scan).
    #[serde(default = "default_true")]
    diff: bool,
    /// `false` for a shape-only call (e.g. an agent-facing limited
    /// preview) that should not record into brainz.
    #[serde(default = "default_true")]
    record: bool,
}

fn default_true() -> bool {
    true
}

fn run_scan(path: &Path, args: &ScanArgs) -> anyhow::Result<(String, Value)> {
    // Scan first; the caller decides whether to record. Keeps the scan
    // pipeline independent of the metrics layer.
    let scope = if args.diff {
        super::scan::Scope::DiffVsHead
    } else {
        super::scan::Scope::Full
    };
    let (report, snapshot, elapsed) = super::scan::run(path, args.threshold, args.limit, scope)?;
    if args.record {
        crate::brainz::record_scan(
            path,
            &snapshot,
            elapsed,
            args.threshold,
            crate::brainz::Origin::Tool,
        );
    }
    let compact = super::compact::tool_report(report);
    Ok((serde_json::to_string_pretty(&compact)?, snapshot))
}

#[cfg(feature = "eyez")]
fn search_docs(args: &Value) -> ToolResult {
    let path = required_str(args, "path")?;
    let query = required_str(args, "query")?;
    let top_k = top_k_arg(args)?;

    let start = Instant::now();
    let result = crate::eyez::Index::open(Path::new(path)).map(|index| {
        let hits = index.search(query, top_k);
        let text = serde_json::to_string(&hits).unwrap_or_else(|_| "[]".to_string());
        (hits, text)
    });
    match result {
        Ok((hits, text)) => {
            let referenced: std::collections::HashSet<&str> =
                hits.iter().map(|h| h.file.as_str()).collect();
            let file_bytes: u64 = referenced
                .iter()
                .filter_map(|f| std::fs::metadata(f).ok())
                .map(|m| m.len())
                .sum();
            crate::brainz::record_search(
                Path::new(path),
                query.len(),
                hits.len(),
                hits.first().map(|h| h.score).unwrap_or(0.0),
                text.len() as u64,
                file_bytes,
                start.elapsed().as_millis() as u64,
            );
            Ok(text_result(text, false))
        }
        Err(err) => Ok(text_result(format!("{err:#}"), true)),
    }
}

#[cfg(feature = "eyez")]
fn top_k_arg(args: &Value) -> Result<std::num::NonZeroUsize, (i64, String)> {
    let Some(value) = args.get("top_k") else {
        return Ok(crate::eyez::DEFAULT_TOP_K);
    };
    value
        .as_u64()
        .and_then(|k| usize::try_from(k).ok())
        .and_then(std::num::NonZeroUsize::new)
        .ok_or_else(|| {
            (
                INVALID_PARAMS,
                "'top_k' must be a positive integer".to_string(),
            )
        })
}

fn explain(args: &Value) -> ToolResult {
    let entries = match args.get("term").and_then(Value::as_str) {
        Some(term) => match crate::noze::glossary::lookup(term) {
            Some(entry) => vec![entry],
            None => {
                let known: Vec<String> = crate::noze::glossary::all()
                    .into_iter()
                    .map(|entry| entry.term)
                    .collect();
                return Ok(text_result(
                    format!("unknown term '{term}'. Known: {}", known.join(", ")),
                    true,
                ));
            }
        },
        None => crate::noze::glossary::all(),
    };
    let text = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string());
    Ok(text_result(text, false))
}

fn triage_finding(args: &Value) -> ToolResult {
    let path = required_str(args, "path")?;
    let pillar = required_str(args, "pillar")?;
    let pattern = required_str(args, "match")?;
    let verdict = required_str(args, "verdict")?;
    let note = args.get("note").and_then(Value::as_str).map(str::to_string);
    let Some(pillar) = crate::report::Pillar::from_name(pillar) else {
        return Ok(text_result(format!("unknown pillar '{pillar}'"), true));
    };
    let Some(verdict) = crate::brainz::TriageVerdict::from_name(verdict) else {
        let accepted = crate::brainz::VERDICTS.join(" | ");
        return Ok(text_result(format!("verdict must be {accepted}"), true));
    };
    match crate::brainz::triage_finding(Path::new(path), pillar, pattern, verdict, note) {
        Ok(labels) => Ok(text_result(
            format!("marked {verdict}: {}", labels.join(" | ")),
            false,
        )),
        Err(err) => Ok(text_result(format!("{err:#}"), true)),
    }
}

fn usage_report(args: &Value) -> ToolResult {
    let path = required_str(args, "path")?;
    let report = crate::brainz::usage_report(Path::new(path));
    serde_json::to_string_pretty(&report)
        .map(|text| text_result(text, false))
        .map_err(|e| (-32603, format!("serializing usage report: {e}")))
}

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;

//! Tool-call handlers for the MCP surface.

use anyhow::Context;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
#[cfg(feature = "eyez")]
use std::time::Instant;

pub(super) type ToolResult = Result<Value, (i64, String)>;

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
        other => Err((-32602, format!("unknown tool: {other}"))),
    }
}

pub(super) fn required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, (i64, String)> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or((-32602, format!("missing '{key}' argument")))
}

pub(super) fn text_result(text: String, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

fn configuration_summary(args: &Value) -> ToolResult {
    let path = required_str(args, "path")?;
    match run_summary_command(path) {
        Ok(text) => Ok(text_result(text, false)),
        Err(err) => Ok(text_result(format!("{err:#}"), true)),
    }
}

fn run_summary_command(path: &str) -> anyhow::Result<String> {
    use std::process::Stdio;
    use std::time::Duration;
    use wait_timeout::ChildExt;

    const SUMMARY_TIMEOUT: Duration = Duration::from_secs(60);

    let exe = std::env::current_exe().context("resolving current executable")?;
    let mut child = Command::new(exe)
        .args(["noze", path, "--summary"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running `sensez noze --summary`")?;
    let stdout_pipe = child.stdout.take().context("capturing summary stdout")?;
    let stderr_pipe = child.stderr.take().context("capturing summary stderr")?;
    let out_reader = read_to_end(stdout_pipe);
    let err_reader = read_to_end(stderr_pipe);

    match child.wait_timeout(SUMMARY_TIMEOUT).context("waiting for summary")? {
        Some(status) => {
            let stdout = out_reader.join().unwrap_or_default();
            let stderr = err_reader.join().unwrap_or_default();
            if !status.success() {
                anyhow::bail!("summary command failed: {stderr}");
            }
            String::from_utf8(stdout.into_bytes())
                .map_err(|_| anyhow::anyhow!("summary command emitted non-UTF-8 output"))
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!(
                "summary command timed out after {}s",
                SUMMARY_TIMEOUT.as_secs()
            )
        }
    }
}

fn read_to_end<R: std::io::Read + Send + 'static>(
    mut pipe: R,
) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = pipe.read_to_string(&mut buf);
        buf
    })
}

fn scan_tool(args: &Value) -> ToolResult {
    let scan_args: ScanArgs = serde_json::from_value(args.clone())
        .map_err(|e| (-32602, format!("invalid arguments: {e}")))?;

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
#[serde(default)]
struct ScanArgs {
    /// Repository path to scan.
    path: String,
    /// Override the duplication token threshold for this scan.
    threshold: Option<usize>,
    /// Per-pillar cap on returned findings (`0` = no cap).
    limit: usize,
    /// Scope the scan to the diff vs. HEAD (`true` is the agent-friendly
    /// default; pass `false` for a full scan).
    diff: bool,
    /// `false` for a shape-only call (e.g. an agent-facing limited
    /// preview) that should not record into brainz.
    record: bool,
}

impl Default for ScanArgs {
    fn default() -> Self {
        Self {
            path: String::new(),
            threshold: None,
            limit: 0,
            diff: true,
            record: true,
        }
    }
}

fn run_scan(path: &Path, args: &ScanArgs) -> anyhow::Result<(String, Value)> {
    // Scan first; the caller decides whether to record. Keeps the scan
    // pipeline independent of the metrics layer.
    let (report, snapshot, elapsed) = if args.diff {
        super::scan::diff(path, args.threshold, args.limit)?
    } else {
        super::scan::full(path, args.threshold, args.limit)?
    };
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
    let top_k = args.get("top_k").and_then(Value::as_u64).unwrap_or(10) as usize;

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

fn explain(args: &Value) -> ToolResult {
    let entries = match args.get("term").and_then(Value::as_str) {
        Some(term) => match crate::noze::glossary::lookup(term) {
            Some(entry) => vec![entry],
            None => return Err((-32602, format!("unknown term '{term}' (omit to list all)"))),
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

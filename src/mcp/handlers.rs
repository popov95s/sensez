//! Tool-call handlers for the MCP surface.

use anyhow::Context;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

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
    let exe = std::env::current_exe().context("resolving current executable")?;
    let output = Command::new(exe)
        .args(["noze", path, "--summary"])
        .output()
        .context("running `sensez noze --summary`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("summary command failed: {stderr}");
    }
    String::from_utf8(output.stdout).context("summary command emitted non-UTF-8 output")
}

fn scan_tool(args: &Value) -> ToolResult {
    let path = required_str(args, "path")?;
    let threshold = args
        .get("threshold")
        .and_then(Value::as_u64)
        .map(|t| t as usize);
    let max = args.get("limit").and_then(Value::as_u64).unwrap_or(0) as usize;
    let diff = scan_diff_arg(args);

    match run_scan(Path::new(path), threshold, max, diff) {
        Ok((text, _snapshot)) => {
            let mut content = vec![json!({"type": "text", "text": text})];
            if let Some(warning) = super::tools::scope_warning(Path::new(path)) {
                content.insert(0, json!({"type": "text", "text": warning}));
            }
            Ok(json!({"content": content, "isError": false}))
        }
        Err(err) => Ok(text_result(format!("{err:#}"), true)),
    }
}

fn scan_diff_arg(args: &Value) -> bool {
    args.get("diff").and_then(Value::as_bool).unwrap_or(true)
}

fn run_scan(
    path: &Path,
    threshold: Option<usize>,
    max: usize,
    diff: bool,
) -> anyhow::Result<(String, Value)> {
    let (report, snapshot) =
        super::scan::run_and_record(path, threshold, max, diff, crate::brainz::Origin::Tool)?;
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
mod tests {
    use super::super::protocol::handle_message;
    use serde_json::{json, Value};

    #[test]
    fn tools_list_includes_metrics_tools() {
        let req = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list"});
        let resp = handle_message(&req).unwrap();
        let names: Vec<&str> = resp["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(names.contains(&"noze_sniff"));
        assert!(names.contains(&"get_configuration_summary"));
        assert!(names.contains(&"brainz_triage"));
        assert!(names.contains(&"brainz_report"));
        assert!(!names.contains(&"record_outcome"));
    }

    #[test]
    fn usage_report_serves_a_clean_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().into_owned();

        let req = json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {
            "name": "brainz_report", "arguments": {"path": path}
        }});
        let resp = handle_message(&req).unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let report: Value = serde_json::from_str(text).unwrap();
        assert!(report.get("session").is_some() && report.get("all_time").is_some());
    }

    #[test]
    fn scan_tool_omits_duplicate_module_noise() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        std::fs::create_dir_all(dir.join("app")).unwrap();
        std::fs::write(dir.join("app.py"), "def flat():\n    return 1\n").unwrap();
        std::fs::write(dir.join("app/__init__.py"), "def pkg():\n    return 2\n").unwrap();
        let path = dir.to_string_lossy().into_owned();

        let req = json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {
            "name": "noze_sniff", "arguments": {"path": path}
        }});
        let resp = handle_message(&req).unwrap();

        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("already defined"));
        assert!(!text.contains("\"issues\""));
        assert!(!text.contains("_total"));
        assert!(!text.contains("\"analyzed_files\""));
        assert!(!text.contains("\"internal_edges\""));
        assert!(!text.contains("\"external_edges\""));
        assert!(!text.contains("\"source_lines\""));
    }

    #[test]
    fn scan_tool_omits_scan_diagnostics() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        let deep = format!("x = {}1{}", "(".repeat(100_000), ")".repeat(100_000));
        std::fs::write(dir.join("too_deep.py"), deep).unwrap();
        let path = dir.to_string_lossy().into_owned();

        let req = json!({"jsonrpc": "2.0", "id": 10, "method": "tools/call", "params": {
            "name": "noze_sniff", "arguments": {"path": path}
        }});
        let resp = handle_message(&req).unwrap();

        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("\"issues\""));
        assert!(!text.contains("\"files_skipped\": 1"));
        assert!(!text.contains("syntax tree deeper than"));
    }

    #[test]
    fn scan_tool_defaults_to_diff_mode() {
        assert!(super::scan_diff_arg(&json!({})));
        assert!(super::scan_diff_arg(&json!({"diff": true})));
        assert!(!super::scan_diff_arg(&json!({"diff": false})));
    }

    #[test]
    fn diff_scan_refreshes_metrics_baseline() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        std::fs::write(dir.join("m.py"), "def f():\n    pass\n").unwrap();
        let path = dir.to_string_lossy().into_owned();

        let req = json!({"jsonrpc": "2.0", "id": 11, "method": "tools/call", "params": {
            "name": "noze_sniff", "arguments": {"path": path, "diff": true}
        }});
        let resp = handle_message(&req).unwrap();

        assert_eq!(resp["result"]["isError"], false);
        assert!(dir.join(".sensez/local-metrics/last-scan.json").exists());
    }
}

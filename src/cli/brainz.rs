//! CLI presentation for local-only Brainz metrics.

use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

pub fn run_report(path: &Path, json: bool) -> Result<()> {
    let report = crate::brainz::usage_report(path);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).context("serializing brainz report")?
        );
    } else {
        println!("{}", render(&report));
    }
    Ok(())
}

fn render(report: &Value) -> String {
    let resolved = pointer_u64(report, "/gate_conversion/resolved")
        .or_else(|| sum_counts(report.pointer("/all_time/resolved_by_detector")));
    let scans = pointer_u64(report, "/all_time/scans").unwrap_or(0);
    let stale = report
        .get("stale_findings")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let mut out = String::from("sensez brainz report\n");
    out.push_str(&format!(
        "  scans recorded: {scans}\n  findings fixed after Sensez reported them: {}\n  stale findings awaiting triage: {stale}\n",
        resolved.unwrap_or(0)
    ));
    if let Some(days) = report
        .pointer("/mean_resolution_days/_overall")
        .and_then(Value::as_f64)
    {
        out.push_str(&format!("  mean time to resolution: {days:.2} day(s)\n"));
    }
    if let Some(privacy) = report.get("privacy").and_then(Value::as_str) {
        out.push_str(&format!("  {privacy}\n"));
    }
    out
}

fn pointer_u64(report: &Value, pointer: &str) -> Option<u64> {
    report.pointer(pointer).and_then(Value::as_u64)
}

fn sum_counts(value: Option<&Value>) -> Option<u64> {
    let object = value?.as_object()?;
    Some(
        object
            .values()
            .filter_map(|entry| entry.get("count").and_then(Value::as_u64))
            .sum(),
    )
}

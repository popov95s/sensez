//! Service scans in a short-lived child process, so a long-lived server hands
//! each scan's memory back to the OS when the child exits.

use crate::report::{AnalysisReport, Severity};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

type Scan = (AnalysisReport, HashMap<String, PathBuf>);

static CHILD_EXE: OnceLock<PathBuf> = OnceLock::new();

#[derive(Serialize, Deserialize)]
struct Payload {
    report: AnalysisReport,
    module_files: HashMap<String, PathBuf>,
    /// Smell fields the report's own serialization leaves out, in finding order.
    smell_extras: Vec<(usize, Severity)>,
}

/// Run every later service scan of this process through `exe mcp analyze`.
pub(crate) fn isolate_service_scans(exe: PathBuf) {
    let _ = CHILD_EXE.set(exe);
}

pub(super) fn exe() -> Option<&'static Path> {
    CHILD_EXE.get().map(PathBuf::as_path)
}

/// Scan in a child; `Ok(None)` when the child could not run or answered in a format this binary does not read.
pub(super) fn analyze(exe: &Path, path: &Path, threshold: Option<usize>) -> Result<Option<Scan>> {
    let mut command = Command::new(exe);
    command.args(["mcp", "analyze"]).arg(path);
    if let Some(threshold) = threshold {
        command.arg("--threshold").arg(threshold.to_string());
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(err) => {
            eprintln!("[sensez] scan child unavailable, scanning in-process: {err}");
            return Ok(None);
        }
    };
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    match serde_json::from_slice::<Payload>(&output.stdout) {
        Ok(payload) => Ok(Some(payload.into_scan())),
        Err(err) => {
            eprintln!("[sensez] unreadable scan child output, scanning in-process: {err}");
            Ok(None)
        }
    }
}

/// The `mcp analyze` side: scan in-process and write the result to stdout.
pub(crate) fn write_service_scan(path: &Path, threshold: Option<usize>) -> Result<()> {
    let (report, module_files) = super::analyze_path_for_child(path, threshold)?;
    let payload = Payload::from_scan(report, module_files);
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &payload).context("writing scan result")?;
    stdout.flush().context("writing scan result")
}

impl Payload {
    fn from_scan(report: AnalysisReport, module_files: HashMap<String, PathBuf>) -> Self {
        let smell_extras = report
            .smells
            .iter()
            .map(|smell| (smell.end_line, smell.severity))
            .collect();
        Payload {
            report,
            module_files,
            smell_extras,
        }
    }

    fn into_scan(mut self) -> Scan {
        for (smell, (end_line, severity)) in self.report.smells.iter_mut().zip(self.smell_extras) {
            smell.end_line = end_line;
            smell.severity = severity;
        }
        (self.report, self.module_files)
    }
}

#[cfg(test)]
#[path = "pipeline_child_tests.rs"]
mod tests;

//! JSON emitter — the programmatic / CI-CD contract.

use crate::report::AnalysisReport;
use anyhow::{Context, Result};

/// Serialize a report to pretty JSON.
pub fn to_json(report: &AnalysisReport) -> Result<String> {
    serde_json::to_string_pretty(report).context("serializing analysis report to JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_empty_report() {
        let json = to_json(&AnalysisReport::default()).unwrap();
        assert!(json.contains("\"cycles\""));
        assert!(json.contains("\"duplication\""));
        assert!(json.contains("\"dead_code\""));
        assert!(json.contains("\"boundaries\""));
    }
}

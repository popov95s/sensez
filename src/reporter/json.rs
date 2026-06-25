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
        assert!(!json.contains("\"unmatched_boundary_rules\""));
        assert!(!json.contains("\"glossary\""));
    }

    #[test]
    fn omits_explanatory_and_placeholder_fields() {
        let mut report = AnalysisReport::default();
        report.meta.glossary = vec![crate::report::GlossaryEntry {
            term: "god_module".into(),
            title: "God Module".into(),
            explanation: "explain only when requested".into(),
        }];
        report.smells.push(crate::report::SmellFinding {
            action: crate::report::ActionLevel::Warning,
            kind: crate::report::SmellKind::GodModule,
            message: "fan-in + fan-out".into(),
            file: "src/config/model.rs".into(),
            line: 0,
            end_line: 0,
            symbol: "src/config/model".into(),
            severity: crate::report::Severity::Warning,
            metric: 26,
            threshold: 25,
            reason: String::new(),
        });

        let json = to_json(&report).unwrap();

        assert!(!json.contains("\"glossary\""));
        assert!(!json.contains("\"unmatched_boundary_rules\""));
        assert!(!json.contains("\"line\": 0"));
    }

    #[test]
    fn omits_scan_diagnostics_by_default() {
        let mut report = AnalysisReport::default();
        report.meta.files_skipped = 1;
        report.meta.issues.push(crate::report::ScanIssue {
            stage: crate::report::ScanStage::Parse,
            file: Some("broken.py".into()),
            message: "parser detail".into(),
        });

        let json = to_json(&report).unwrap();

        assert!(!json.contains("\"files_skipped\""));
        assert!(!json.contains("\"issues\""));
        assert!(!json.contains("parser detail"));
    }
}

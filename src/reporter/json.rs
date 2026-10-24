//! JSON emitter — the programmatic / CI-CD contract.

use super::ScanDiagnostics;
use crate::report::AnalysisReport;
use anyhow::{Context, Result};

/// Serialize a report to pretty JSON, with or without its scan diagnostics.
pub fn to_json(report: &AnalysisReport, diagnostics: ScanDiagnostics) -> Result<String> {
    let serialized = match diagnostics {
        ScanDiagnostics::Show => serde_json::to_string_pretty(report),
        ScanDiagnostics::Hide => serde_json::to_string_pretty(&without_diagnostics(report)),
    };
    serialized.context("serializing analysis report to JSON")
}

fn without_diagnostics(report: &AnalysisReport) -> AnalysisReport {
    let mut hidden = report.clone();
    hidden.meta.issues.clear();
    hidden.meta.files_skipped = 0;
    hidden
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_empty_report() {
        let json = to_json(&AnalysisReport::default(), ScanDiagnostics::Hide).unwrap();
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
            lines: Vec::new(),
            symbol: "src/config/model".into(),
            severity: crate::report::Severity::Warning,
            metric: 26,
            threshold: 25,
            reason: String::new(),
        });

        let json = to_json(&report, ScanDiagnostics::Hide).unwrap();

        assert!(!json.contains("\"glossary\""));
        assert!(!json.contains("\"unmatched_boundary_rules\""));
        assert!(!json.contains("\"line\": 0"));
        assert!(json.contains("\"action\": \"warning\""));
        assert!(!json.contains("\"severity\""));
    }

    fn report_with_skipped_file() -> AnalysisReport {
        let mut report = AnalysisReport::default();
        report.meta.files_skipped = 1;
        report.meta.issues.push(crate::report::ScanIssue {
            stage: crate::report::ScanStage::Parse,
            file: Some("broken.py".into()),
            message: "parser detail".into(),
        });
        report
    }

    #[test]
    fn hidden_diagnostics_omit_issues_and_skip_count() {
        let json = to_json(&report_with_skipped_file(), ScanDiagnostics::Hide).unwrap();

        assert!(!json.contains("\"files_skipped\""));
        assert!(!json.contains("\"issues\""));
        assert!(!json.contains("parser detail"));
    }

    #[test]
    fn shown_diagnostics_list_issues_and_skip_count() {
        let json = to_json(&report_with_skipped_file(), ScanDiagnostics::Show).unwrap();

        assert!(json.contains("\"files_skipped\": 1"));
        assert!(json.contains("parser detail"));
    }
}

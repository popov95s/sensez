use super::settings::{DisplayLevel, Settings};
use crate::report::{ActionLevel, AnalysisReport};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use url::Url;

const SOURCE: &str = "sensez";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDiagnostics {
    pub uri: Url,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub range: Range,
    pub severity: u8,
    pub code: String,
    pub source: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_information: Vec<RelatedInformation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedInformation {
    pub location: Location,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub uri: Url,
    pub range: Range,
}

struct FindingDetails {
    start_line: usize,
    end_line: usize,
    action: ActionLevel,
    code: String,
    message: String,
    related_information: Vec<RelatedInformation>,
}

pub fn from_report(report: &AnalysisReport) -> BTreeMap<String, Vec<Diagnostic>> {
    let mut diagnostics = BTreeMap::new();
    for finding in &report.cycles {
        for edge in &finding.edges {
            let related = finding
                .edges
                .iter()
                .filter(|other| other.file != edge.file || other.line != edge.line)
                .filter_map(|other| related(&other.file, other.line, "Cycle edge"))
                .collect();
            push(
                &mut diagnostics,
                &edge.file,
                FindingDetails {
                    start_line: edge.line,
                    end_line: edge.line,
                    action: finding.action,
                    code: "cycle/import".to_owned(),
                    message: format!(
                        "Circular import: {} imports {}",
                        edge.from_module, edge.to_module
                    ),
                    related_information: related,
                },
            );
        }
    }
    for finding in &report.dead_code {
        push(
            &mut diagnostics,
            &finding.file,
            FindingDetails {
                start_line: finding.line,
                end_line: finding.line,
                action: finding.action,
                code: "dead-code/symbol".to_owned(),
                message: format!("Possibly unused {} `{}`", finding.kind, finding.symbol),
                related_information: Vec::new(),
            },
        );
    }
    for finding in &report.boundaries {
        push(
            &mut diagnostics,
            &finding.file,
            FindingDetails {
                start_line: finding.line,
                end_line: finding.line,
                action: finding.action,
                code: "boundary/import".to_owned(),
                message: format!(
                    "Boundary `{}`: {} must not import {}",
                    finding.rule, finding.from_module, finding.to_module
                ),
                related_information: Vec::new(),
            },
        );
    }
    for clone in &report.duplication {
        for occurrence in &clone.occurrences {
            let related = clone
                .occurrences
                .iter()
                .filter(|other| {
                    other.file != occurrence.file || other.start_row != occurrence.start_row
                })
                .filter_map(|other| related(&other.file, other.start_row, "Matching duplicate"))
                .collect();
            push(
                &mut diagnostics,
                &occurrence.file,
                FindingDetails {
                    start_line: occurrence.start_row,
                    end_line: occurrence.end_row,
                    action: clone.action,
                    code: "duplication/structural".to_owned(),
                    message: format!("Structural duplicate ({} tokens)", clone.token_length),
                    related_information: related,
                },
            );
        }
    }
    for finding in &report.smells {
        push(
            &mut diagnostics,
            &finding.file,
            FindingDetails {
                start_line: finding.line,
                end_line: finding.end_line,
                action: finding.action,
                code: format!("smell/{}", finding.kind),
                message: finding.message.clone(),
                related_information: Vec::new(),
            },
        );
    }
    diagnostics
}

pub fn retain_visible(report: &mut AnalysisReport, settings: Settings) {
    let visible = |action| is_visible(action, settings.level);
    report.cycles.retain(|finding| visible(finding.action));
    report.dead_code.retain(|finding| visible(finding.action));
    report.boundaries.retain(|finding| visible(finding.action));
    report.duplication.retain(|finding| visible(finding.action));
    report.smells.retain(|finding| visible(finding.action));
}

fn is_visible(action: ActionLevel, level: DisplayLevel) -> bool {
    match level {
        DisplayLevel::MustFix => action == ActionLevel::MustFix,
        DisplayLevel::Warning => action <= ActionLevel::Warning,
        DisplayLevel::Advisory => action <= ActionLevel::Advisory,
        DisplayLevel::Info => true,
        DisplayLevel::Off => false,
    }
}

fn push(output: &mut BTreeMap<String, Vec<Diagnostic>>, file: &Path, details: FindingDetails) {
    let Some(uri) = file_uri(file) else { return };
    output.entry(uri).or_default().push(Diagnostic {
        range: range(details.start_line, details.end_line),
        severity: severity(details.action),
        code: details.code,
        source: SOURCE,
        message: details.message,
        related_information: details.related_information,
    });
}

fn related(file: &Path, line: usize, message: &str) -> Option<RelatedInformation> {
    let uri = Url::parse(&file_uri(file)?).ok()?;
    Some(RelatedInformation {
        location: Location {
            uri,
            range: range(line, line),
        },
        message: message.to_owned(),
    })
}

fn file_uri(file: &Path) -> Option<String> {
    let absolute = file
        .canonicalize()
        .ok()
        .unwrap_or_else(|| file.to_path_buf());
    Url::from_file_path(absolute).ok().map(Into::into)
}

fn range(start: usize, end: usize) -> Range {
    let start_line = start.saturating_sub(1);
    let end_line = end.max(start_line.saturating_add(1));
    Range {
        start: Position {
            line: start_line,
            character: 0,
        },
        end: Position {
            line: end_line,
            character: 0,
        },
    }
}

fn severity(action: ActionLevel) -> u8 {
    match action {
        ActionLevel::MustFix => 1,
        ActionLevel::Warning => 2,
        ActionLevel::Advisory => 3,
        ActionLevel::Info => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_lines_are_clamped_to_the_first_line() {
        let range = range(0, 0);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.end.line, 1);
    }

    #[test]
    fn action_levels_map_to_lsp_severity() {
        assert_eq!(severity(ActionLevel::MustFix), 1);
        assert_eq!(severity(ActionLevel::Info), 4);
    }

    #[test]
    fn off_never_publishes_diagnostics() {
        assert!(!is_visible(ActionLevel::MustFix, DisplayLevel::Off));
    }
}

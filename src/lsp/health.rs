use crate::report::{ActionLevel, AnalysisReport, SmellKind};
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSummary {
    pub root: String,
    pub scope: &'static str,
    pub current_changes: ChangeCount,
    pub cycles: PillarCount,
    pub dead_code: PillarCount,
    pub boundaries: PillarCount,
    pub duplication: PillarCount,
    pub smells: PillarCount,
    pub cycle_findings: Vec<HealthFinding>,
    pub dead_code_findings: Vec<HealthFinding>,
    pub boundary_findings: Vec<HealthFinding>,
    pub duplication_findings: Vec<HealthFinding>,
    pub smell_findings: Vec<HealthFinding>,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCount {
    pub total: usize,
    pub blocking: usize,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PillarCount {
    pub total: usize,
    pub must_fix: usize,
    pub warning: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthFinding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub label: String,
    pub detail: String,
    pub file: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
}

impl HealthSummary {
    pub fn from_report(root: String, scope: &'static str, report: &AnalysisReport) -> Self {
        Self {
            root,
            scope,
            current_changes: ChangeCount::default(),
            cycles: count(report.cycles.iter().map(|finding| finding.action)),
            dead_code: count(report.dead_code.iter().map(|finding| finding.action)),
            boundaries: count(report.boundaries.iter().map(|finding| finding.action)),
            duplication: count(report.duplication.iter().map(|finding| finding.action)),
            smells: count(report.smells.iter().map(|finding| finding.action)),
            cycle_findings: report
                .cycles
                .iter()
                .enumerate()
                .flat_map(|(index, finding)| {
                    let group = format!("Cycle {} ({} imports)", index + 1, finding.edges.len());
                    finding.edges.iter().map(move |edge| HealthFinding {
                        group: Some(group.clone()),
                        label: format!("{} imports {}", edge.from_module, edge.to_module),
                        detail: "Circular import".to_owned(),
                        file: edge.file.display().to_string(),
                        line: edge.line,
                        end_line: None,
                    })
                })
                .collect(),
            dead_code_findings: report
                .dead_code
                .iter()
                .map(|finding| HealthFinding {
                    group: None,
                    label: format!("Possibly unused {} `{}`", finding.kind, finding.symbol),
                    detail: format!("{:?} confidence", finding.confidence),
                    file: finding.file.display().to_string(),
                    line: finding.line,
                    end_line: None,
                })
                .collect(),
            boundary_findings: report
                .boundaries
                .iter()
                .map(|finding| HealthFinding {
                    group: None,
                    label: format!(
                        "{} must not import {}",
                        finding.from_module, finding.to_module
                    ),
                    detail: format!("Boundary `{}`", finding.rule),
                    file: finding.file.display().to_string(),
                    line: finding.line,
                    end_line: None,
                })
                .collect(),
            duplication_findings: report
                .duplication
                .iter()
                .enumerate()
                .flat_map(|(index, finding)| {
                    let locations = finding.occurrences.len();
                    let group = format!(
                        "Duplicate {} ({} tokens, {} locations)",
                        index + 1,
                        finding.token_length,
                        locations
                    );
                    finding
                        .occurrences
                        .iter()
                        .enumerate()
                        .map(move |(index, occurrence)| HealthFinding {
                            group: Some(group.clone()),
                            label: format!("Location {}/{}", index + 1, locations),
                            detail: "Structural duplicate".to_owned(),
                            file: occurrence.file.display().to_string(),
                            line: occurrence.start_row,
                            end_line: Some(occurrence.end_row),
                        })
                })
                .collect(),
            smell_findings: report
                .smells
                .iter()
                .map(|finding| HealthFinding {
                    group: Some(smell_group(finding.kind)),
                    label: finding.message.clone(),
                    detail: format!("{} `{}`", finding.kind, finding.symbol),
                    file: finding.file.display().to_string(),
                    line: finding.line,
                    end_line: None,
                })
                .collect(),
        }
    }

    pub fn set_current_changes(&mut self, report: &AnalysisReport) {
        self.current_changes = ChangeCount {
            total: report.cycles.len()
                + report.dead_code.len()
                + report.boundaries.len()
                + report.duplication.len()
                + report.smells.len(),
            blocking: report
                .cycles
                .iter()
                .map(|finding| finding.action)
                .chain(report.dead_code.iter().map(|finding| finding.action))
                .chain(report.boundaries.iter().map(|finding| finding.action))
                .chain(report.duplication.iter().map(|finding| finding.action))
                .chain(report.smells.iter().map(|finding| finding.action))
                .filter(|action| *action == ActionLevel::MustFix)
                .count(),
        };
    }
}

fn smell_group(kind: SmellKind) -> String {
    kind.as_str()
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn count(actions: impl Iterator<Item = ActionLevel>) -> PillarCount {
    actions.fold(PillarCount::default(), |mut count, action| {
        count.total += 1;
        match action {
            ActionLevel::MustFix => count.must_fix += 1,
            ActionLevel::Warning => count.warning += 1,
            ActionLevel::Advisory | ActionLevel::Info => {}
        }
        count
    })
}

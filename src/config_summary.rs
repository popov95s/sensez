//! Aggregated scan summary for configuration tuning surfaces.

use crate::config::Config;
use crate::report::{AnalysisReport, Pillar, SkipCounts, SmellKind};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct ConfigurationSummary {
    total_count: usize,
    by_rule: BTreeMap<String, RuleSummary>,
    scope: Scope,
}

#[derive(Debug, Serialize)]
struct Scope {
    duplication: PillarScope,
    smells: PillarScope,
}

/// Files a source-level pillar leaves out, and why.
#[derive(Debug, Serialize)]
struct PillarScope {
    exclude_effective: Vec<String>,
    include_tests: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_generated: Option<bool>,
    skipped: SkipCounts,
}

#[derive(Debug, Serialize)]
struct RuleSummary {
    rule_id: String,
    count: usize,
    current_threshold: Value,
    config_path: String,
    sample_file_paths: Vec<String>,
}

impl RuleSummary {
    fn new(rule_id: impl Into<String>, current_threshold: Value, config_path: &str) -> Self {
        RuleSummary {
            rule_id: rule_id.into(),
            count: 0,
            current_threshold,
            config_path: config_path.to_string(),
            sample_file_paths: Vec::new(),
        }
    }

    fn add(&mut self, path: impl Into<Option<String>>) {
        self.count += 1;
        if let Some(path) = path.into() {
            if self.sample_file_paths.len() < 2 && !self.sample_file_paths.contains(&path) {
                self.sample_file_paths.push(path);
            }
        }
    }
}

/// Per-rule summary JSON of an already selected report.
pub(crate) fn render(path: &Path, report: &AnalysisReport, config: &Config) -> Result<String> {
    let summary = from_report(path, report, config);
    serde_json::to_string_pretty(&summary).context("serializing configuration summary")
}

fn from_report(root: &Path, report: &AnalysisReport, config: &Config) -> ConfigurationSummary {
    let mut by_rule = BTreeMap::new();
    add_cycles(root, report, &mut by_rule);
    add_dead_code(root, report, config, &mut by_rule);
    add_boundaries(root, report, config, &mut by_rule);
    add_duplication(root, report, config, &mut by_rule);
    add_smells(root, report, &mut by_rule);
    let total_count = by_rule.values().map(|rule| rule.count).sum();
    ConfigurationSummary {
        total_count,
        by_rule,
        scope: scope(report, config),
    }
}

fn scope(report: &AnalysisReport, config: &Config) -> Scope {
    Scope {
        duplication: PillarScope {
            exclude_effective: config.duplication.exclude.clone(),
            include_tests: config.duplication.include_tests,
            include_generated: Some(config.duplication.include_generated),
            skipped: report.meta.duplication_skipped,
        },
        smells: PillarScope {
            exclude_effective: config.smells.exclude.clone(),
            include_tests: config.smells.include_tests,
            include_generated: None,
            skipped: report.meta.smells_skipped,
        },
    }
}

fn add_cycles(root: &Path, report: &AnalysisReport, by_rule: &mut BTreeMap<String, RuleSummary>) {
    let samples = report.cycles.iter().map(|cycle| {
        cycle
            .edges
            .first()
            .map(|edge| sample_path(root, &edge.file))
    });
    add_pillar(
        by_rule,
        Pillar::Cycles,
        Value::Null,
        "action.cycles",
        samples,
    );
}

fn add_dead_code(
    root: &Path,
    report: &AnalysisReport,
    config: &Config,
    by_rule: &mut BTreeMap<String, RuleSummary>,
) {
    let threshold = json!({
        "unused_imports": config.dead_code.unused_imports,
        "unused_methods": config.dead_code.unused_methods,
        "unused_properties": config.dead_code.unused_properties,
        "unused_variables": config.dead_code.unused_variables
    });
    let samples = report
        .dead_code
        .iter()
        .map(|finding| Some(sample_path(root, &finding.file)));
    add_pillar(by_rule, Pillar::DeadCode, threshold, "dead_code", samples);
}

fn add_boundaries(
    root: &Path,
    report: &AnalysisReport,
    config: &Config,
    by_rule: &mut BTreeMap<String, RuleSummary>,
) {
    for finding in &report.boundaries {
        let threshold = boundary_config(&finding.rule, config);
        let rule = by_rule
            .entry(finding.rule.clone())
            .or_insert_with(|| RuleSummary::new(&finding.rule, threshold, "boundaries.forbidden"));
        rule.add(sample_path(root, &finding.file));
    }
}

fn add_duplication(
    root: &Path,
    report: &AnalysisReport,
    config: &Config,
    by_rule: &mut BTreeMap<String, RuleSummary>,
) {
    let samples = report.duplication.iter().map(|clone| {
        clone
            .occurrences
            .first()
            .map(|occurrence| sample_path(root, &occurrence.file))
    });
    let threshold = json!(config.duplication.threshold);
    add_pillar(
        by_rule,
        Pillar::Duplication,
        threshold,
        "duplication.threshold",
        samples,
    );
}

/// One rule per pillar, counted from its findings' sample paths; left out when nothing was found.
fn add_pillar(
    by_rule: &mut BTreeMap<String, RuleSummary>,
    pillar: Pillar,
    threshold: Value,
    config_path: &str,
    samples: impl Iterator<Item = Option<String>>,
) {
    let id = pillar.as_str();
    let rule = by_rule
        .entry(id.into())
        .or_insert_with(|| RuleSummary::new(id, threshold, config_path));
    for sample in samples {
        rule.add(sample);
    }
    if rule.count == 0 {
        by_rule.remove(id);
    }
}

fn add_smells(root: &Path, report: &AnalysisReport, by_rule: &mut BTreeMap<String, RuleSummary>) {
    let thresholds = smell_thresholds(report);
    for finding in &report.smells {
        let rule_id = finding.kind.as_str();
        let threshold = thresholds
            .get(&finding.kind)
            .map(threshold_value)
            .unwrap_or(Value::Null);
        let rule = by_rule.entry(rule_id.to_string()).or_insert_with(|| {
            RuleSummary::new(rule_id, threshold, &format!("smells.rules.{rule_id}"))
        });
        rule.add(sample_path(root, &finding.file));
    }
}

fn smell_thresholds(report: &AnalysisReport) -> BTreeMap<SmellKind, BTreeSet<u32>> {
    let mut thresholds: BTreeMap<SmellKind, BTreeSet<u32>> = BTreeMap::new();
    for finding in &report.smells {
        thresholds
            .entry(finding.kind)
            .or_default()
            .insert(finding.threshold);
    }
    thresholds
}

fn threshold_value(values: &BTreeSet<u32>) -> Value {
    if values.len() == 1 {
        json!(values.iter().next().copied().unwrap_or_default())
    } else {
        json!(values.iter().copied().collect::<Vec<_>>())
    }
}

fn boundary_config(rule: &str, config: &Config) -> Value {
    config
        .boundaries
        .forbidden
        .iter()
        .find(|candidate| format!("{} -x-> {}", candidate.from, candidate.to) == rule)
        .map(|candidate| json!({"from": candidate.from, "to": candidate.to}))
        .unwrap_or(Value::Null)
}

fn sample_path(root: &Path, file: &Path) -> String {
    relative_path(root, file).to_string_lossy().into_owned()
}

fn relative_path(root: &Path, file: &Path) -> PathBuf {
    file.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| file.to_path_buf())
}

#[cfg(test)]
mod tests;

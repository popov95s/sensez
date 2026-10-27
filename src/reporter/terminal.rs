//! Human-readable colored terminal renderer (a view over [`AnalysisReport`]).

use crate::report::{ActionLevel, AnalysisReport, Confidence, Pillar, ReportMode, SkipCounts};
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

pub fn render(report: &AnalysisReport, explain: bool) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{}",
        "sensez — structural maintainability report".bold()
    );
    if report.meta.mode == ReportMode::Diff {
        let _ = writeln!(
            out,
            "{}",
            "diff mode: only findings touching changed lines".dimmed()
        );
    }
    if report.meta.files_skipped > 0
        && super::ScanDiagnostics::from_env() == super::ScanDiagnostics::Show
    {
        let _ = writeln!(
            out,
            "{}",
            format!(
                "⚠ {} scan issue(s) reduced fidelity — findings may be incomplete",
                report.meta.files_skipped
            )
            .yellow()
        );
        for issue in &report.meta.issues {
            match &issue.file {
                Some(file) => {
                    let _ = writeln!(
                        out,
                        "    {} {}: {}",
                        issue.stage,
                        file.display(),
                        issue.message
                    );
                }
                None => {
                    let _ = writeln!(out, "    {}: {}", issue.stage, issue.message);
                }
            }
        }
    }

    section(
        &mut out,
        report,
        Pillar::Cycles,
        "Circular imports",
        report.cycles.len(),
        report.meta.cycles_total,
    );
    for cycle in &report.cycles {
        let _ = writeln!(
            out,
            "    [{}] {} {}",
            action_label(cycle.action),
            "↻".yellow(),
            cycle.modules.join(" → ")
        );
        for edge in &cycle.edges {
            let _ = writeln!(
                out,
                "        {}:{}  {} → {}",
                edge.file.display(),
                edge.line,
                edge.from_module,
                edge.to_module
            );
        }
        if let Some(hint) = &cycle.hint {
            let _ = writeln!(out, "        {}", hint.dimmed());
        }
    }

    if section(
        &mut out,
        report,
        Pillar::Duplication,
        "Duplication",
        report.duplication.len(),
        report.meta.duplication_total,
    ) {
        write_skipped(
            &mut out,
            &report.meta.duplication_skipped,
            "duplication.include_tests / include_generated / exclude",
        );
    }
    for class in &report.duplication {
        let detail = class
            .hint
            .clone()
            .unwrap_or_else(|| format!("structural clone: {} tokens", class.token_length));
        let _ = writeln!(out, "    [{}] {}:", action_label(class.action), detail);
        for occ in &class.occurrences {
            let _ = writeln!(
                out,
                "      {}:{}-{}",
                occ.file.display(),
                occ.start_row,
                occ.end_row
            );
        }
    }

    section(
        &mut out,
        report,
        Pillar::DeadCode,
        "Dead code candidates",
        report.dead_code.len(),
        report.meta.dead_code_total,
    );
    for finding in &report.dead_code {
        let _ = write!(
            out,
            "    [{}] {}  {}::{} ({}) [{}]",
            action_label(finding.action),
            location(&finding.file, finding.line),
            finding.module,
            finding.symbol,
            finding.kind,
            confidence_label(finding.confidence),
        );
        if !finding.reason.is_empty() {
            let _ = write!(out, " {}", finding.reason);
        }
        out.push('\n');
    }

    if section(
        &mut out,
        report,
        Pillar::Smells,
        "Code smells",
        report.smells.len(),
        report.meta.smells_total,
    ) {
        write_skipped(
            &mut out,
            &report.meta.smells_skipped,
            "smells.include_tests / exclude",
        );
    }
    let mut seen_suggestions: HashSet<String> = HashSet::new();
    let shown_by_kind: HashMap<&str, usize> = {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for smell in &report.smells {
            *counts.entry(smell.kind.as_str()).or_insert(0) += 1;
        }
        counts
    };
    for (index, finding) in report.smells.iter().enumerate() {
        let kind = finding.kind.as_str();
        let starts_kind = index == 0 || report.smells[index - 1].kind != finding.kind;
        if starts_kind {
            let shown = shown_by_kind.get(kind).copied().unwrap_or(0);
            let total = report.meta.smell_totals.get(kind).copied().unwrap_or(shown);
            let suffix = if shown < total {
                format!(" {}", format!("(showing top {shown})").dimmed())
            } else {
                String::new()
            };
            let _ = writeln!(out, "  {} ({}){}", kind.cyan(), total, suffix);
        }
        let smell_loc = location(&finding.file, finding.line);
        let (summary, suggestion) = split_smell_message(&finding.message);
        let rendered_message = match suggestion {
            Some(suggestion) => {
                let key = format!("{}::{suggestion}", finding.kind.as_str());
                if seen_suggestions.insert(key) {
                    finding.message.as_str()
                } else {
                    summary
                }
            }
            None => finding.message.as_str(),
        };
        let _ = writeln!(
            out,
            "    [{}] {}  {} — {}",
            action_label(finding.action),
            smell_loc,
            display_symbol(&finding.symbol),
            rendered_message
        );
    }

    boundaries_header(&mut out, report);
    for violation in &report.boundaries {
        let _ = writeln!(
            out,
            "    [{}] {}:{}  {} → {} [{}]",
            action_label(violation.action),
            violation.file.display(),
            violation.line,
            violation.from_module,
            violation.to_module,
            violation.rule
        );
    }
    for rule in &report.meta.unmatched_boundary_rules {
        let _ = writeln!(
            out,
            "    {} rule matched no module (check the pattern): {}",
            "⚠".yellow(),
            rule
        );
    }

    if explain && !report.meta.glossary.is_empty() {
        let _ = writeln!(out, "\n{}", "What these mean".bold());
        for g in &report.meta.glossary {
            let _ = writeln!(out, "  {} — {}", g.title.cyan(), g.explanation.dimmed());
        }
    }
    out
}

/// The pillar's heading with its count; false when the run filtered the pillar out.
fn section(
    out: &mut String,
    report: &AnalysisReport,
    pillar: Pillar,
    title: &str,
    shown: usize,
    total: usize,
) -> bool {
    if report.meta.filtered_pillars.contains(&pillar) {
        return false;
    }
    let count_str = total.to_string();
    let colored = if total == 0 {
        count_str.green()
    } else {
        count_str.red()
    };
    let suffix = if shown < total {
        format!(" {}", format!("(showing top {shown})").dimmed())
    } else {
        String::new()
    };
    let _ = writeln!(out, "\n{} ({}){}", title.bold().cyan(), colored, suffix);
    true
}

fn boundaries_header(out: &mut String, report: &AnalysisReport) {
    if report.meta.filtered_pillars.contains(&Pillar::Boundaries) {
        return;
    }
    if report.meta.boundaries_configured {
        section(
            out,
            report,
            Pillar::Boundaries,
            "Boundary violations",
            report.boundaries.len(),
            report.meta.boundaries_total,
        );
    } else {
        let _ = writeln!(
            out,
            "\n{} ({})",
            "Boundary violations".bold().cyan(),
            "not configured".dimmed()
        );
    }
}

/// One dimmed line naming how many files a pillar skipped and the knobs that control it.
fn write_skipped(out: &mut String, skipped: &SkipCounts, knobs: &str) {
    if skipped.is_empty() {
        return;
    }
    let line = format!(
        "skipped {} test, {} generated and {} excluded file(s) ({knobs})",
        skipped.tests, skipped.generated, skipped.config_exclude
    );
    let _ = writeln!(out, "    {}", line.dimmed());
}

fn confidence_label(confidence: Confidence) -> colored::ColoredString {
    match confidence {
        Confidence::High => "high".red(),
        Confidence::Medium => "medium".yellow(),
        Confidence::Low => "low".dimmed(),
    }
}

fn location(file: &std::path::Path, line: usize) -> String {
    if line > 0 {
        format!("{}:{}", file.display(), line)
    } else {
        file.display().to_string()
    }
}

fn display_symbol(symbol: &str) -> &str {
    if symbol.is_empty() {
        crate::spine::ir::ANONYMOUS
    } else {
        symbol
    }
}

fn split_smell_message(message: &str) -> (&str, Option<&str>) {
    match message.split_once(" — ") {
        Some((summary, suggestion)) => (summary, Some(suggestion)),
        None => (message, None),
    }
}

fn action_label(level: ActionLevel) -> colored::ColoredString {
    match level {
        ActionLevel::MustFix => "must_fix".red().bold(),
        ActionLevel::Warning => "warning".yellow(),
        ActionLevel::Advisory => "advisory".cyan(),
        ActionLevel::Info => "info".dimmed(),
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;

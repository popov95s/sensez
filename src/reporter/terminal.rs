//! Human-readable colored terminal renderer (a view over [`AnalysisReport`]).

use crate::report::{ActionLevel, AnalysisReport, Confidence, Severity};
use colored::Colorize;
use std::fmt::Write;

pub fn render(report: &AnalysisReport, explain: bool) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{}",
        "sensez — structural maintainability report".bold()
    );
    if report.meta.files_skipped > 0 {
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
    }

    section(
        &mut out,
        "Duplication",
        report.duplication.len(),
        report.meta.duplication_total,
    );
    for class in &report.duplication {
        let detail = class.hint.as_deref().unwrap_or("structural clone");
        if class.token_length > 0 {
            let _ = writeln!(
                out,
                "    [{}] {} ({}):",
                action_label(class.action),
                detail,
                class.token_length.to_string().bold()
            );
        } else {
            let _ = writeln!(out, "    [{}] {}:", action_label(class.action), detail);
        }
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
        "Dead code candidates",
        report.dead_code.len(),
        report.meta.dead_code_total,
    );
    for finding in &report.dead_code {
        let loc = if finding.line > 0 {
            format!("{}:{}", finding.file.display(), finding.line)
        } else {
            finding.file.display().to_string()
        };
        let _ = writeln!(
            out,
            "    [{}] {}  {}::{} ({}) [{}]",
            action_label(finding.action),
            loc,
            finding.module,
            finding.symbol,
            finding.kind,
            confidence_label(finding.confidence)
        );
    }

    section(
        &mut out,
        "Code smells",
        report.smells.len(),
        report.meta.smells_total,
    );
    for finding in &report.smells {
        let loc = if finding.line > 0 {
            format!("{}:{}", finding.file.display(), finding.line)
        } else {
            finding.file.display().to_string()
        };
        let _ = writeln!(
            out,
            "    {} {}  {} ({}) — {}",
            smell_labels(finding.action, finding.severity),
            loc,
            finding.symbol,
            finding.kind,
            finding.message
        );
    }

    if report.meta.boundaries_configured {
        section(
            &mut out,
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

fn section(out: &mut String, title: &str, shown: usize, total: usize) {
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
}

fn confidence_label(confidence: Confidence) -> colored::ColoredString {
    match confidence {
        Confidence::High => "high".red(),
        Confidence::Medium => "medium".yellow(),
        Confidence::Low => "low".dimmed(),
    }
}

fn severity_label(severity: Severity) -> colored::ColoredString {
    match severity {
        Severity::Critical => "critical".red(),
        Severity::Warning => "warning".yellow(),
        Severity::Info => "info".dimmed(),
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn smell_labels(action: ActionLevel, severity: Severity) -> String {
    if action.as_str() == severity_name(severity) {
        format!("[{}]", action_label(action))
    } else {
        format!("[{}] [{}]", action_label(action), severity_label(severity))
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

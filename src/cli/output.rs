//! CLI-only report shaping: pillar selection plus concise default output.

use super::spec::ScanOptions;
use crate::report::{self, AnalysisReport, Confidence, LimitPolicy, Pillar, SmellCap};
use std::io::{self, Write};

const DEFAULT_PILLAR_TOP: usize = 5;
const DEFAULT_SMELL_KIND_TOP: usize = 3;

pub fn print_line(text: &str) -> io::Result<()> {
    let result = write_stdout(|out| {
        out.write_all(text.as_bytes())?;
        out.write_all(b"\n")
    });
    match result {
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => other,
    }
}

fn write_stdout(f: impl FnOnce(&mut io::StdoutLock<'_>) -> io::Result<()>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    f(&mut out)?;
    out.flush()
}

/// Keeps only the findings this run is about: requested pillars, no low-confidence dead code.
pub fn select(report: &mut AnalysisReport, options: &ScanOptions) {
    apply_pillar_filter(report, options);
    report
        .dead_code
        .retain(|finding| finding.confidence != Confidence::Low);
    report.refresh_totals();
}

/// Truncates selected findings for display and attaches the glossary.
pub fn shape(report: &mut AnalysisReport, options: &ScanOptions) {
    if !options.all {
        report::limit(
            report,
            LimitPolicy {
                pillar_max: options.max.unwrap_or(DEFAULT_PILLAR_TOP),
                smells: SmellCap::PerKind(options.max.unwrap_or(DEFAULT_SMELL_KIND_TOP)),
            },
        );
    }
    report.meta.glossary = crate::noze::glossary::for_report(report);
}

fn apply_pillar_filter(report: &mut AnalysisReport, options: &ScanOptions) {
    let any = options.duplicates
        || options.dead_code
        || options.cycles
        || options.boundaries
        || options.smells;
    if !any {
        return;
    }
    if !options.cycles {
        report.cycles.clear();
        report.meta.filtered_pillars.push(Pillar::Cycles);
    }
    if !options.dead_code {
        report.dead_code.clear();
        report.meta.filtered_pillars.push(Pillar::DeadCode);
    }
    if !options.boundaries {
        report.boundaries.clear();
        report.meta.filtered_pillars.push(Pillar::Boundaries);
    }
    if !options.duplicates {
        report.duplication.clear();
        report.meta.filtered_pillars.push(Pillar::Duplication);
    }
    if !options.smells {
        report.smells.clear();
        report.meta.filtered_pillars.push(Pillar::Smells);
    }
}

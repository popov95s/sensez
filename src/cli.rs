//! Command-line dispatch.

mod spec;

use crate::diff::ChangedLines;
use anyhow::{Context, Result};
use clap::Parser;
use spec::{Cli, Command, FailOnNewLevel, NozeAction, NozeArgs, ScanOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Command::Noze(args) => run_noze(args),
        Command::Scan { path, options } => run_scan(&path, &options),
        Command::Explain { term } => run_explain(term.as_deref()).map(|()| ExitCode::SUCCESS),
        Command::Init {
            path,
            agent,
            gate,
            no_metrics,
            yes,
        } => crate::setup::run(crate::setup::InitOptions {
            path,
            agent,
            gate,
            no_metrics,
            yes,
        })
        .map(|()| ExitCode::SUCCESS),
        #[cfg(feature = "mcp")]
        Command::Mcp(args) => match args.action {
            spec::McpAction::Serve => serve_mcp(),
        },
        #[cfg(feature = "mcp")]
        Command::Serve => serve_mcp(),
        #[cfg(feature = "eyez")]
        Command::Eyez(args) => match args.action {
            spec::EyezAction::Search {
                path,
                query,
                top_k,
                json,
            } => run_search(&path, &query, top_k, json).map(|()| ExitCode::SUCCESS),
        },
        #[cfg(feature = "eyez")]
        Command::Search {
            path,
            query,
            top_k,
            json,
        } => run_search(&path, &query, top_k, json).map(|()| ExitCode::SUCCESS),
    }
}

fn run_noze(args: NozeArgs) -> Result<ExitCode> {
    match args.action {
        Some(NozeAction::Sniff { path, options }) => {
            let path = path.unwrap_or_else(|| PathBuf::from("."));
            run_scan(&path, &options)
        }
        Some(NozeAction::Explain { term }) => {
            run_explain(term.as_deref()).map(|()| ExitCode::SUCCESS)
        }
        None => {
            let path = args.path.unwrap_or_else(|| PathBuf::from("."));
            run_scan(&path, &args.options)
        }
    }
}

#[cfg(feature = "mcp")]
fn serve_mcp() -> Result<ExitCode> {
    tokio::runtime::Runtime::new()
        .context("starting tokio runtime")?
        .block_on(crate::mcp::serve())
        .map(|()| ExitCode::SUCCESS)
}

/// Build/refresh the eyez index for `path` and print the top hits for `query`.
#[cfg(feature = "eyez")]
fn run_search(path: &Path, query: &str, top_k: usize, json: bool) -> Result<()> {
    let index = crate::eyez::Index::open(path)?;
    let hits = index.search(query, top_k);
    if json {
        println!("{}", serde_json::to_string_pretty(&hits)?);
    } else if hits.is_empty() {
        println!(
            "No indexed documentation matched (index size: {}).",
            index.len()
        );
    } else {
        for hit in &hits {
            println!(
                "{:.3}  {}:{}  {}  [{:?}]\n        {}",
                hit.score, hit.file, hit.line, hit.symbol_path, hit.kind, hit.text
            );
        }
    }
    Ok(())
}

/// Print the plain-English definition of one finding category, or all of them.
fn run_explain(term: Option<&str>) -> Result<()> {
    use crate::noze::glossary;
    match term {
        Some(t) => match glossary::lookup(t) {
            Some(e) => println!("{} ({})\n  {}", e.title, e.term, e.explanation),
            None => {
                let known: Vec<String> = glossary::all().into_iter().map(|e| e.term).collect();
                anyhow::bail!("unknown term '{t}'. Known: {}", known.join(", "));
            }
        },
        None => {
            for e in glossary::all() {
                println!("{} ({})\n  {}\n", e.title, e.term, e.explanation);
            }
        }
    }
    Ok(())
}

fn run_scan(path: &Path, options: &ScanOptions) -> Result<ExitCode> {
    if options.summary {
        println!("{}", crate::config_summary::scan(path, options.threshold)?);
        return Ok(ExitCode::SUCCESS);
    }
    let diff = build_diff(path, options.diff, options.diff_from.as_deref());
    let mut report = crate::analyze_path(path, options.threshold, diff.changed.as_ref())?;
    report.meta.issues.extend(diff.issues);
    report.meta.files_skipped = report.meta.issues.len();
    crate::output_filter::apply(&mut report, path, &options.output_glob)
        .context("applying output glob filter")?;
    crate::noze::limit(&mut report, options.max);

    let output = if options.json {
        crate::reporter::to_json(&report)?
    } else {
        crate::reporter::render(&report, options.explain)
    };
    println!("{output}");

    if let Some(level) = options.fail_on_new {
        if report.meta.mode == crate::noze::ReportMode::Diff
            && report_meets_fail_level(&report, level)
        {
            return Ok(ExitCode::FAILURE);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn report_meets_fail_level(report: &crate::report::AnalysisReport, level: FailOnNewLevel) -> bool {
    let threshold = match level {
        FailOnNewLevel::MustFix => crate::report::ActionLevel::MustFix,
        FailOnNewLevel::Warning => crate::report::ActionLevel::Warning,
        FailOnNewLevel::Advisory => crate::report::ActionLevel::Advisory,
        FailOnNewLevel::Info => crate::report::ActionLevel::Info,
    };
    report_findings(report).any(|action| action <= threshold)
}

fn report_findings(
    report: &crate::report::AnalysisReport,
) -> impl Iterator<Item = crate::report::ActionLevel> + '_ {
    report
        .cycles
        .iter()
        .map(|finding| finding.action)
        .chain(report.dead_code.iter().map(|finding| finding.action))
        .chain(report.boundaries.iter().map(|finding| finding.action))
        .chain(report.duplication.iter().map(|finding| finding.action))
        .chain(report.smells.iter().map(|finding| finding.action))
}

struct DiffSelection {
    changed: Option<ChangedLines>,
    issues: Vec<crate::report::ScanIssue>,
}

/// Resolve the optional diff source into a [`ChangedLines`] set.
fn build_diff(path: &Path, diff: bool, diff_from: Option<&str>) -> DiffSelection {
    if diff {
        return match crate::diff::git::changed_vs_head(path) {
            Ok(changed) => DiffSelection {
                changed: Some(changed),
                issues: Vec::new(),
            },
            Err(err) => DiffSelection {
                changed: None,
                issues: vec![diff_issue(None, format!("{err:#}"))],
            },
        };
    }
    if let Some(src) = diff_from {
        let text = if src == "-" {
            let mut buf = String::new();
            match std::io::stdin().read_to_string(&mut buf) {
                Ok(_) => buf,
                Err(err) => {
                    return DiffSelection {
                        changed: None,
                        issues: vec![diff_issue(
                            None,
                            format!("reading unified diff from stdin: {err}"),
                        )],
                    };
                }
            }
        } else {
            match std::fs::read_to_string(src) {
                Ok(text) => text,
                Err(err) => {
                    return DiffSelection {
                        changed: None,
                        issues: vec![diff_issue(
                            Some(PathBuf::from(src)),
                            format!("reading diff file {src}: {err}"),
                        )],
                    };
                }
            }
        };
        return DiffSelection {
            changed: Some(ChangedLines::from_unified(&text, path)),
            issues: Vec::new(),
        };
    }
    DiffSelection {
        changed: None,
        issues: Vec::new(),
    }
}

fn diff_issue(file: Option<PathBuf>, message: String) -> crate::report::ScanIssue {
    crate::report::ScanIssue {
        stage: crate::report::ScanStage::Diff,
        file,
        message,
    }
}
#[cfg(test)]
mod tests;

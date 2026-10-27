//! Command-line dispatch.

#[cfg(feature = "mcp")]
mod brainz;
pub(crate) mod output;
pub(crate) mod spec;

use crate::diff::ChangedLines;
use anyhow::{Context, Result};
use clap::Parser;
use spec::{Cli, Command, FailOnNewLevel, NozeAction, NozeArgs, ScanOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    run_inner(cli)
}

fn run_inner(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Some(Command::Noze(args)) => run_noze(args),
        Some(Command::Reflexez(args)) => crate::reflexez::run(args),
        Some(Command::Init {
            path,
            agent,
            gate,
            no_metrics,
            yes,
        }) => crate::setup::run(crate::setup::InitOptions {
            path,
            agent,
            gate,
            no_metrics,
            yes,
        })
        .map(|()| ExitCode::SUCCESS),
        #[cfg(feature = "mcp")]
        Some(Command::Mcp(args)) => match args.action {
            spec::McpAction::Serve => serve_mcp(),
        },
        #[cfg(feature = "lsp")]
        Some(Command::Server(args)) => match args.action {
            spec::ServerAction::Stdio => crate::lsp::serve().map(|()| ExitCode::SUCCESS),
        },
        #[cfg(feature = "mcp")]
        Some(Command::Brainz(args)) => match args.action {
            spec::BrainzAction::Report { path, json } => {
                brainz::run_report(&path.unwrap_or_else(|| PathBuf::from(".")), json)
                    .map(|()| ExitCode::SUCCESS)
            }
        },
        #[cfg(feature = "eyez")]
        Some(Command::Eyez(args)) => match args.action {
            spec::EyezAction::Search { args, top_k, json } => {
                let (path, query) = match args.as_slice() {
                    [path, query] => (PathBuf::from(path), query.as_str()),
                    [query] => (PathBuf::from("."), query.as_str()),
                    _ => anyhow::bail!("eyez search takes [PATH] QUERY"),
                };
                run_search(&path, query, top_k, json).map(|()| ExitCode::SUCCESS)
            }
            spec::EyezAction::Reindex {
                path,
                force,
                semantic,
            } => run_reindex(&path.unwrap_or_else(|| PathBuf::from(".")), force, semantic)
                .map(|()| ExitCode::SUCCESS),
        },
        None => {
            let path = cli.path.unwrap_or_else(|| PathBuf::from("."));
            run_scan(&path, &cli.options)
        }
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
    eprintln!("sensez MCP server running on stdio");
    tokio::runtime::Runtime::new()
        .context("starting tokio runtime")?
        .block_on(crate::mcp::serve())
        .map(|()| ExitCode::SUCCESS)
}

#[cfg(feature = "eyez")]
fn run_reindex(path: &Path, force: bool, semantic: bool) -> Result<()> {
    let report = crate::eyez::reindex(path, force, semantic)?;
    output::print_line(&format!(
        "Indexed {} doc/comment item(s).{}",
        report.docs,
        if report.semantic_warmed {
            " Semantic duplication cache warmed."
        } else {
            ""
        }
    ))?;
    Ok(())
}

/// Build/refresh the eyez index for `path` and print the top hits for `query`.
#[cfg(feature = "eyez")]
fn run_search(path: &Path, query: &str, top_k: std::num::NonZeroUsize, json: bool) -> Result<()> {
    let index = crate::eyez::Index::open(path)?;
    let hits = index.search(query, top_k);
    if json {
        output::print_line(&serde_json::to_string_pretty(&hits)?)?;
    } else if hits.is_empty() {
        output::print_line(&format!(
            "No indexed documentation matched (index size: {}).",
            index.len()
        ))?;
    } else {
        for hit in &hits {
            output::print_line(&format!(
                "{:.3}  {}:{}  {}  [{:?}]\n        {}",
                hit.score, hit.file, hit.line, hit.symbol_path, hit.kind, hit.text
            ))?;
        }
    }
    Ok(())
}

/// Print the plain-English definition of one finding category, or all of them.
fn run_explain(term: Option<&str>) -> Result<()> {
    use crate::noze::glossary;
    match term {
        Some(t) => match glossary::lookup(t) {
            Some(e) => {
                output::print_line(&format!("{} ({})\n  {}", e.title, e.term, e.explanation))?
            }
            None => {
                let known: Vec<String> = glossary::all().into_iter().map(|e| e.term).collect();
                anyhow::bail!("unknown term '{t}'. Known: {}", known.join(", "));
            }
        },
        None => {
            for e in glossary::all() {
                output::print_line(&format!("{} ({})\n  {}", e.title, e.term, e.explanation))?;
                output::print_line("")?;
            }
        }
    }
    Ok(())
}

fn run_scan(path: &Path, options: &ScanOptions) -> Result<ExitCode> {
    let (mut report, config) = selected_report(path, options)?;
    let report_started = std::time::Instant::now();
    let gate_fails = gate_fails(&report, options);
    let rendered = if options.summary {
        crate::config_summary::render(path, &report, &config)?
    } else {
        output::shape(&mut report, options);
        render_report(&report, options)?
    };
    if std::env::var_os("SENSEZ_TIMING").is_some() {
        eprintln!(
            "[timing] report           {:>7.1}ms",
            report_started.elapsed().as_secs_f64() * 1e3
        );
    }
    output::print_line(&rendered)?;

    ensure_complete_for_gate(&report, options)?;
    if let Some(note) = gate_config_note(&report, options) {
        eprintln!("{note}");
    }
    if gate_fails {
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

/// Analyze `path` and keep what this run asked for: diff scope, output globs, pillars.
fn selected_report(
    path: &Path,
    options: &ScanOptions,
) -> Result<(crate::report::AnalysisReport, crate::config::Config)> {
    let wants_diff = options.diff || options.diff_from.is_some() || options.fail_on_new.is_some();
    if options.fail_on_new.is_some() && !options.diff && options.diff_from.is_none() {
        eprintln!(
            "note: --fail-on-new implies --diff (findings limited to working-tree changes vs HEAD)"
        );
    }
    let changed = build_diff(
        path,
        wants_diff && options.diff_from.is_none(),
        options.diff_from.as_deref(),
    )
    .context(
        "diff mode was requested but no diff could be read; refusing to fall back to a full scan \
         (run inside a git work tree, pass --diff-from FILE, or drop --diff/--fail-on-new)",
    )?;
    let (mut report, module_files, config) =
        crate::pipeline::analyze_path_with_config(path, options.threshold)?;
    if let Some(changed) = changed.as_ref() {
        crate::diff::apply(&mut report, changed, &module_files);
    }
    for issue in &report.meta.issues {
        match &issue.file {
            Some(file) => eprintln!(
                "[sensez] {}: {}: {}",
                issue.stage,
                file.display(),
                issue.message
            ),
            None => eprintln!("[sensez] {}: {}", issue.stage, issue.message),
        }
    }
    crate::reporter::apply(&mut report, path, &options.output_glob)
        .context("applying output glob filter")?;
    output::select(&mut report, options);
    Ok((report, config))
}

fn render_report(report: &crate::report::AnalysisReport, options: &ScanOptions) -> Result<String> {
    if options.json {
        return crate::reporter::to_json(report, crate::reporter::ScanDiagnostics::from_env());
    }
    Ok(crate::reporter::render(report, options.explain))
}

/// A gated run fails when an analyzer was skipped, instead of passing on partial results.
fn ensure_complete_for_gate(
    report: &crate::report::AnalysisReport,
    options: &ScanOptions,
) -> Result<()> {
    if options.fail_on.is_none() && options.fail_on_new.is_none() {
        return Ok(());
    }
    match report
        .meta
        .issues
        .iter()
        .find(|issue| issue.stage == crate::report::ScanStage::Analyze)
    {
        Some(issue) => anyhow::bail!("incomplete scan: {}", issue.message),
        None => Ok(()),
    }
}

/// Gate summary when configuration problems fell back to defaults; the gate still decides.
fn gate_config_note(
    report: &crate::report::AnalysisReport,
    options: &ScanOptions,
) -> Option<String> {
    if options.fail_on.is_none() && options.fail_on_new.is_none() {
        return None;
    }
    let config_issues = report
        .meta
        .issues
        .iter()
        .filter(|issue| issue.stage == crate::report::ScanStage::Config)
        .count();
    (config_issues > 0).then(|| {
        format!(
            "[sensez] gate: result computed despite {config_issues} configuration issue(s) reported above"
        )
    })
}

fn gate_fails(report: &crate::report::AnalysisReport, options: &ScanOptions) -> bool {
    let full = options
        .fail_on
        .is_some_and(|level| report_meets_fail_level(report, level));
    let new = options.fail_on_new.is_some_and(|level| {
        report.meta.mode == crate::report::ReportMode::Diff
            && report_meets_fail_level(report, level)
    });
    full || new
}

fn report_meets_fail_level(report: &crate::report::AnalysisReport, level: FailOnNewLevel) -> bool {
    let threshold = match level {
        FailOnNewLevel::MustFix => crate::report::ActionLevel::MustFix,
        FailOnNewLevel::Warning => crate::report::ActionLevel::Warning,
        FailOnNewLevel::Advisory => crate::report::ActionLevel::Advisory,
        FailOnNewLevel::Info => crate::report::ActionLevel::Info,
    };
    report.actions().any(|action| action <= threshold)
}

/// Resolve the optional diff source into a [`ChangedLines`] set.
fn build_diff(path: &Path, diff: bool, diff_from: Option<&str>) -> Result<Option<ChangedLines>> {
    if diff {
        return crate::diff::git::changed_vs_head(path).map(Some);
    }
    let Some(src) = diff_from else {
        return Ok(None);
    };
    let text = if src == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("reading unified diff from stdin")?;
        buf
    } else {
        std::fs::read_to_string(src).with_context(|| format!("reading diff file {src}"))?
    };
    Ok(Some(ChangedLines::from_unified(&text, path)))
}
#[cfg(test)]
mod gate_filter_tests;
#[cfg(test)]
mod gate_tests;
#[cfg(test)]
mod parse_tests;
#[cfg(test)]
mod reflexez_tests;
#[cfg(test)]
mod summary_tests;
#[cfg(test)]
mod tests;

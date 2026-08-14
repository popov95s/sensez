//! End-to-end scan orchestration shared by the CLI and the Python surface.

use crate::config::model::Config;
use crate::noze;
use crate::profiles::registry;
use crate::report::{AnalysisReport, ScanStage};
use crate::reporter::{self, Format};
use crate::spine::parser::ParsedFile;
use crate::spine::{crawler, graph};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[path = "pipeline_session.rs"]
mod session;
use session::AnalysisSession;

static SERVICE_SESSION: OnceLock<AnalysisSession> = OnceLock::new();

/// Crawl, parse, build the graph, run analyzers, apply triaged suppressions
/// and precision ranking. Returns the report and a module→file map (needed for
/// diff filtering via [`crate::diff::apply`]).
pub fn analyze_path(
    path: &Path,
    threshold: Option<usize>,
) -> Result<(AnalysisReport, HashMap<String, PathBuf>)> {
    analyze_path_inner(path, threshold, None)
}

fn analyze_path_in_session(
    session: &AnalysisSession,
    path: &Path,
    threshold: Option<usize>,
) -> Result<(AnalysisReport, HashMap<String, PathBuf>)> {
    analyze_path_inner(path, threshold, Some(session))
}

pub(crate) fn analyze_path_in_service(
    path: &Path,
    threshold: Option<usize>,
) -> Result<(AnalysisReport, HashMap<String, PathBuf>)> {
    let session = SERVICE_SESSION.get_or_init(AnalysisSession::default);
    analyze_path_in_service_with_session(session, path, threshold)
}

fn analyze_path_in_service_with_session(
    session: &AnalysisSession,
    path: &Path,
    threshold: Option<usize>,
) -> Result<(AnalysisReport, HashMap<String, PathBuf>)> {
    let (config, _) = Config::load_for_scan(path);
    if config.cache_enabled() {
        analyze_path_in_session(session, path, threshold)
    } else {
        analyze_path(path, threshold)
    }
}

fn analyze_path_inner(
    path: &Path,
    threshold: Option<usize>,
    session: Option<&AnalysisSession>,
) -> Result<(AnalysisReport, HashMap<String, PathBuf>)> {
    let (mut config, config_issues) = Config::load_for_scan(path);
    if let Some(value) = threshold {
        config.duplication.threshold = value;
    }
    let mut timer = PhaseTimer::start();
    crate::spine::parser::timing::reset();
    let discovery = crawler::discover(path, &config.exclude, &|p| {
        crate::profiles::registry::should_parse_path(p)
    })
    .with_context(|| format!("crawling {}", path.display()))?;
    timer.lap("crawl");
    let project = if session.is_some() && config_issues.is_empty() && discovery.issues.is_empty() {
        crate::spine::cache::load_project(&discovery.files).ok()
    } else {
        None
    };
    observe_source_state(path, config.signature(), project.as_ref());
    if project.is_some() {
        timer.lap("fingerprint");
    } else {
        timer.cache_disabled();
    }
    let mut changed_paths = Vec::new();
    let parsed = match (session, project.as_ref()) {
        (Some(session), Some(project)) => {
            let (parsed, stats) = session.parse(path, project);
            changed_paths = stats.changed_paths.clone();
            timer.incremental_cache(stats, project.sources.len());
            parsed
        }
        _ => crate::spine::parser::parse_files(&discovery.files),
    };
    timer.lap("parse");
    timer.parse_breakdown(crate::spine::parser::timing::take());
    config.dead_code.entry_modules = entry_modules(path, &parsed.files);
    let graph = graph::build(&parsed.files, &config.roots);
    timer.lap("graph");
    if !changed_paths.is_empty() {
        let impact = crate::spine::impact::affected_files(
            &graph,
            &changed_paths,
            crate::spine::impact::ImpactOptions::default(),
        );
        timer.graph_impact(&impact);
    }
    let mut report = noze::run_with_root(&parsed.files, &graph, &config, Some(path));
    report.meta.issues.extend(config_issues);
    report.meta.issues.extend(discovery.issues);
    debug_assert_eq!(
        discovery.skipped,
        report
            .meta
            .issues
            .iter()
            .filter(|issue| issue.stage == ScanStage::Discover)
            .count()
    );
    report.meta.issues.extend(parsed.issues);
    report.meta.files_skipped = report.meta.issues.len();
    timer.lap("analyze");

    let mut module_files: HashMap<String, PathBuf> = HashMap::new();
    for idx in graph.graph.node_indices() {
        let n = &graph.graph[idx];
        if n.is_external {
            continue;
        }
        module_files
            .entry(n.module_name.clone())
            .or_insert_with(|| n.file_path.clone());
    }

    crate::brainz::apply_suppressions(path, &mut report);
    crate::brainz::rank_by_precision(path, &mut report);

    Ok((report, module_files))
}

fn observe_source_state(
    root: &Path,
    config_hash: u64,
    project: Option<&crate::spine::cache::ProjectInputs>,
) {
    let Some(project) = project else {
        return;
    };
    let manifest = crate::source_state::SourceManifest::from_root_hashes(
        root,
        project
            .sources
            .iter()
            .map(|source| (source.path.clone(), source.content_hash)),
    );
    crate::brainz::observe_source_state(
        root,
        crate::source_state::SourceState::new(
            1,
            config_hash,
            crate::diff::git::current_branch(root),
            manifest,
        ),
    );
}

/// Opt-in per-phase tracing (`SENSEZ_TIMING=1`).
pub(super) struct PhaseTimer {
    enabled: bool,
    start: std::time::Instant,
    last: std::time::Instant,
}

impl PhaseTimer {
    fn start() -> Self {
        let now = std::time::Instant::now();
        Self {
            enabled: std::env::var_os("SENSEZ_TIMING").is_some(),
            start: now,
            last: now,
        }
    }

    fn lap(&mut self, label: &str) {
        if !self.enabled {
            return;
        }
        let now = std::time::Instant::now();
        eprintln!(
            "[timing] {label:<8} {:>7.1}ms  (cumulative {:.1}ms)",
            (now - self.last).as_secs_f64() * 1e3,
            (now - self.start).as_secs_f64() * 1e3,
        );
        self.last = now;
    }

    fn cache_disabled(&self) {
        if self.enabled {
            eprintln!("[timing] analysis-cache disabled");
        }
    }

    fn incremental_cache(&self, stats: crate::spine::cache::ChangeStats, total: usize) {
        if self.enabled {
            eprintln!(
                "[timing] parse-cache reused={}/{} bytes={}/{} added={} modified={} deleted={} unchanged={}",
                stats.reusable,
                total,
                stats.reusable_bytes,
                stats.total_bytes,
                stats.added,
                stats.modified,
                stats.deleted,
                stats.unchanged,
            );
        }
    }

    fn graph_impact(&self, impact: &crate::spine::impact::AffectedFiles) {
        if self.enabled {
            eprintln!(
                "[timing] graph-impact callers={} callees={} total={} unmapped={}",
                impact.dependents.files.len(),
                impact.dependencies.files.len(),
                impact.all_files().len(),
                impact.unmapped().len(),
            );
        }
    }

    fn parse_breakdown(&self, breakdown: crate::spine::parser::timing::Breakdown) {
        if !self.enabled {
            return;
        }
        let ms = |duration: std::time::Duration| duration.as_secs_f64() * 1e3;
        eprintln!(
            "[timing] parse-work read={:.1}ms hash={:.1}ms tree={:.1}ms depth={:.1}ms walk={:.1}ms (CPU-sum)",
            ms(breakdown.read),
            ms(breakdown.hash),
            ms(breakdown.tree),
            ms(breakdown.depth),
            ms(breakdown.walk),
        );
    }
}

/// Best-effort manifest entry points for each language present in the scan.
fn entry_modules(project_root: &Path, parsed: &[ParsedFile]) -> Vec<String> {
    let languages: HashSet<_> = parsed.iter().map(|f| f.language).collect();
    languages
        .into_iter()
        .flat_map(|lang| registry::dead_code_profile(lang).entry_modules(project_root))
        .collect()
}

/// Run and render a scan. `max = 0` leaves findings uncapped.
pub fn scan(path: &Path, threshold: Option<usize>, format: Format, max: usize) -> Result<String> {
    let (mut report, _module_files) = analyze_path(path, threshold)?;
    noze::limit(&mut report, max);
    match format {
        Format::Json => reporter::to_json(&report),
        Format::Terminal => Ok(reporter::render(&report, false)),
    }
}

#[cfg(test)]
#[path = "pipeline_session_tests.rs"]
mod session_tests;
#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod tests;

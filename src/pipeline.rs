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

#[path = "pipeline_cache.rs"]
mod cache;
use cache::{parse_available, persist_analysis, PersistRequest};

/// Crawl, parse, build the graph, run analyzers, apply triaged suppressions
/// and precision ranking. Returns the report and a module→file map (needed for
/// diff filtering via [`crate::diff::apply`]).
pub fn analyze_path(
    path: &Path,
    threshold: Option<usize>,
) -> Result<(AnalysisReport, HashMap<String, PathBuf>)> {
    analyze_path_inner(path, threshold)
}

fn analyze_path_inner(
    path: &Path,
    threshold: Option<usize>,
) -> Result<(AnalysisReport, HashMap<String, PathBuf>)> {
    let (mut config, config_issues) = Config::load_for_scan(path);
    if let Some(value) = threshold {
        config.duplication.threshold = value;
    }
    let mut timer = PhaseTimer::start();
    let discovery = crawler::discover(path, &config.exclude, &|p| {
        crate::profiles::registry::should_parse_path(p)
    })
    .with_context(|| format!("crawling {}", path.display()))?;
    timer.lap("crawl");
    let cache_enabled = config.cache_enabled();
    let parse_cache = cache_enabled.then(|| crate::spine::cache::ParseCache::new(path));
    let project = if cache_enabled && config_issues.is_empty() && discovery.issues.is_empty() {
        crate::spine::cache::load_project(&discovery.files).ok()
    } else {
        None
    };
    observe_source_state(path, config.signature(), project.as_ref());
    if cache_enabled {
        timer.lap("fingerprint");
    } else {
        timer.cache_disabled();
    }
    if cache_enabled {
        timer.cache_hit(false);
    }
    let (mut parsed, parse_stats) = parse_available(
        &discovery.files,
        project.as_ref(),
        parse_cache.as_ref(),
        &mut timer,
    );
    timer.lap("parse");
    config.dead_code.entry_modules = entry_modules(path, &parsed.files);
    let graph = graph::build(&parsed.files, &config.roots);
    timer.lap("graph");
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

    persist_analysis(
        PersistRequest {
            is_cli: true,
            project,
            parse_cache,
            cacheable: report.meta.issues.is_empty(),
            parse_stats,
        },
        &mut parsed.files,
    );
    timer.lap("cache-queue");
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

    fn cache_hit(&self, hit: bool) {
        if self.enabled {
            eprintln!(
                "[timing] analysis-cache {}",
                if hit { "hit" } else { "miss" }
            );
        }
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
#[path = "pipeline_tests.rs"]
mod tests;

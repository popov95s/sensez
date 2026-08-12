//! End-to-end scan orchestration shared by the CLI and the Python surface.

use crate::config::model::Config;
use crate::noze;
use crate::profiles::registry;
use crate::report::{AnalysisReport, ScanStage};
use crate::reporter::{self, Format};
use crate::spine::parser::ParsedFile;
use crate::spine::{crawler, graph, parser};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Crawl, parse, build the graph, run analyzers, apply triaged suppressions
/// and precision ranking. Returns the report and a module→file map (needed for
/// diff filtering via [`crate::diff::apply`]).
pub fn analyze_path(
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
    let snapshot_cache = crate::spine::cache::SnapshotCache::new(path);
    let project = if config_issues.is_empty() && discovery.issues.is_empty() {
        crate::spine::cache::load_project(&discovery.files, config.signature()).ok()
    } else {
        None
    };
    timer.lap("fingerprint");
    if let Some(snapshot) = project
        .as_ref()
        .and_then(|project| snapshot_cache.load(project.key))
    {
        timer.cache_hit(true);
        let mut report = snapshot.report;
        crate::brainz::apply_suppressions(path, &mut report);
        crate::brainz::rank_by_precision(path, &mut report);
        return Ok((report, snapshot.module_files));
    }
    timer.cache_hit(false);
    let parsed = match project.as_ref() {
        Some(project) => parser::parse_sources(&project.sources),
        None => parser::parse_files(&discovery.files),
    };
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

    if let Some(key) = project
        .as_ref()
        .map(|project| project.key)
        .filter(|_| report.meta.issues.is_empty())
    {
        let snapshot =
            crate::spine::cache::AnalysisSnapshot::new(report.clone(), module_files.clone());
        crate::spine::cache::persist_snapshot(snapshot_cache, key, snapshot);
    }
    timer.lap("cache-queue");
    crate::brainz::apply_suppressions(path, &mut report);
    crate::brainz::rank_by_precision(path, &mut report);

    Ok((report, module_files))
}

/// Opt-in per-phase tracing (`SENSEZ_TIMING=1`).
struct PhaseTimer {
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

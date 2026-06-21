//! End-to-end scan orchestration shared by the CLI and the Python surface.

use crate::config::model::Config;
use crate::diff::ChangedLines;
use crate::noze;
use crate::profiles::registry;
use crate::report::{AnalysisReport, ScanIssue, ScanStage};
use crate::reporter::{self, Format};
use crate::spine::parser::ParsedFile;
use crate::spine::{crawler, graph, parser};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Crawl, parse, build the graph, run analyzers, then apply optional diff scope.
pub fn analyze_path(
    path: &Path,
    threshold: Option<usize>,
    diff: Option<&ChangedLines>,
) -> Result<AnalysisReport> {
    let mut config = Config::load(path).context("loading sensez.toml")?;
    if let Some(value) = threshold {
        config.duplication.threshold = value;
    }
    let timer = PhaseTimer::start();
    let discovery = crawler::discover(path, &config.exclude)
        .with_context(|| format!("crawling {}", path.display()))?;
    timer.lap("crawl");
    let parsed = parser::parse_files(&discovery.files);
    timer.lap("parse");
    config.dead_code.entry_modules = entry_modules(path, &parsed.files);
    let graph = graph::build(&parsed.files, &config.roots);
    timer.lap("graph");
    let mut report = noze::run(&parsed.files, &graph, &config);
    report.meta.issues.extend(parsed.issues);
    report
        .meta
        .issues
        .extend(graph.duplicate_modules.iter().map(|dup| ScanIssue {
            stage: ScanStage::Graph,
            file: Some(dup.duplicate_file.clone()),
            message: format!(
                "module `{}` already defined at {}",
                dup.module_name,
                dup.first_file.display()
            ),
        }));
    report.meta.files_skipped = discovery.skipped + report.meta.issues.len();
    timer.lap("analyze");

    if let Some(changed) = diff {
        let module_files: HashMap<String, PathBuf> = graph
            .graph
            .node_indices()
            .map(|i| &graph.graph[i])
            .filter(|n| !n.is_external)
            .map(|n| (n.module_name.clone(), n.file_path.clone()))
            .collect();
        crate::diff::apply(&mut report, changed, &module_files);
    }
    Ok(report)
}

/// Opt-in per-phase tracing (`SENSEZ_TIMING=1`).
struct PhaseTimer {
    start: Option<std::time::Instant>,
    last: std::cell::Cell<std::time::Instant>,
}

impl PhaseTimer {
    fn start() -> Self {
        let on = std::env::var_os("SENSEZ_TIMING").is_some();
        let now = std::time::Instant::now();
        PhaseTimer {
            start: on.then_some(now),
            last: std::cell::Cell::new(now),
        }
    }

    fn lap(&self, label: &str) {
        if let Some(start) = self.start {
            let now = std::time::Instant::now();
            eprintln!(
                "[timing] {label:<8} {:>7.1}ms  (cumulative {:.1}ms)",
                (now - self.last.get()).as_secs_f64() * 1e3,
                (now - start).as_secs_f64() * 1e3,
            );
            self.last.set(now);
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
    let mut report = analyze_path(path, threshold, None)?;
    noze::limit(&mut report, max);
    match format {
        Format::Json => reporter::to_json(&report),
        Format::Terminal => Ok(reporter::render(&report, false)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_produces_json() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("m.py"), "def f():\n    pass\n").unwrap();

        let json = scan(&dir, Some(50), Format::Json, 0).unwrap();
        assert!(json.contains("\"duplication\""));
    }

    #[test]
    fn diff_mode_filters_to_touched_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        // Two modules, each with an unreferenced top-level function.
        fs::write(dir.join("touched.py"), "def unused_here():\n    return 1\n").unwrap();
        fs::write(
            dir.join("other.py"),
            "def unused_elsewhere():\n    return 2\n",
        )
        .unwrap();

        // Pretend only touched.py (its def line) was changed.
        let mut changed = ChangedLines::default();
        changed.add(&dir.join("touched.py"), 1, 2);

        let report = analyze_path(&dir, None, Some(&changed)).unwrap();
        assert_eq!(report.meta.mode, crate::noze::ReportMode::Diff);
        let dead: Vec<_> = report.dead_code.iter().map(|f| f.symbol.as_str()).collect();
        assert!(dead.contains(&"unused_here"), "touched file's finding kept");
        assert!(
            !dead.contains(&"unused_elsewhere"),
            "untouched file's finding dropped"
        );
        assert!(report
            .dead_code
            .iter()
            .all(|f| f.reason == "added_unreferenced"));
    }

    /// `--diff` scopes smells by the symbol's BODY span, not just its `def`
    /// line: editing code inside a function (even one you didn't originally
    /// write) surfaces its smell, while an edit entirely outside it does not.
    /// Pinned on synthetic code (NOT from any real repo).
    #[test]
    fn diff_smell_scoping_covers_the_function_body() {
        use crate::noze::SmellKind;

        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        // `split_variable` is opt-in; enable just it to keep the report focused.
        fs::write(dir.join("sensez.toml"), "[smells]\nsplit_variable = true\n").unwrap();
        // `result` is plainly reassigned 3x (default threshold) -> SplitVariable
        // on `proc` (def line 1, body lines 2-5). `untouched` is a second fn so
        // an "elsewhere" edit has somewhere to land.
        let src = "def proc(data):\n    \
                   result = []\n    \
                   result = step_one(data)\n    \
                   result = step_two(result)\n    \
                   return result\n\n\
                   def untouched():\n    \
                   return 0\n";
        let file = dir.join("m.py");
        fs::write(&file, src).unwrap();

        let has_split = |changed: &ChangedLines| {
            analyze_path(&dir, None, Some(changed))
                .unwrap()
                .smells
                .iter()
                .any(|s| s.kind == SmellKind::SplitVariable && s.symbol == "proc")
        };

        // Sanity: a full scan finds it (proves the fixture triggers the smell).
        assert!(
            analyze_path(&dir, None, None)
                .unwrap()
                .smells
                .iter()
                .any(|s| s.kind == SmellKind::SplitVariable),
            "fixture must produce a split_variable smell"
        );

        // Body-only edit (line 4, inside proc but not its def): now SURFACED.
        let mut body_only = ChangedLines::default();
        body_only.add(&file, 4, 4);
        assert!(
            has_split(&body_only),
            "editing the function body must surface its smell"
        );

        // Edit entirely outside proc (the other function): not relevant.
        let mut elsewhere = ChangedLines::default();
        elsewhere.add(&file, 8, 8);
        assert!(
            !has_split(&elsewhere),
            "an edit outside the function must NOT surface its smell"
        );
    }

    #[test]
    fn parse_failures_surface_concrete_scan_issues() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        let deep = format!("x = {}1{}", "(".repeat(100_000), ")".repeat(100_000));
        fs::write(dir.join("too_deep.py"), deep).unwrap();

        let report = analyze_path(&dir, None, None).unwrap();
        assert_eq!(report.meta.files_skipped, 1);
        assert_eq!(report.meta.issues.len(), 1);
        assert_eq!(report.meta.issues[0].stage, crate::report::ScanStage::Parse);
        assert!(
            report.meta.issues[0]
                .message
                .contains("syntax tree deeper than"),
            "{:?}",
            report.meta.issues[0]
        );
    }

    #[test]
    fn action_policy_is_applied_to_pillars_and_smells() {
        use crate::noze::SmellKind;

        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("sensez.toml"),
            "[action]\ndead_code = \"info\"\n\
             [smells.rules.long_function]\nmax_lines = 2\naction = \"must_fix\"\n",
        )
        .unwrap();
        fs::write(
            dir.join("m.py"),
            "def unused_long():\n    x = 1\n    y = 2\n    z = 3\n    return x + y + z\n",
        )
        .unwrap();

        let report = analyze_path(&dir, None, None).unwrap();
        let dead = report
            .dead_code
            .iter()
            .find(|finding| finding.symbol == "unused_long")
            .expect("unused function should be reported");
        assert_eq!(dead.action, crate::report::ActionLevel::Info);

        let smell = report
            .smells
            .iter()
            .find(|finding| finding.kind == SmellKind::LongFunction)
            .expect("long function should be reported");
        assert_eq!(smell.action, crate::report::ActionLevel::MustFix);
    }
}

use crate::noze::{
    AnalysisReport, BoundaryViolation, CloneClass, CycleFinding, DeadCodeFinding, SmellFinding,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

type RepoKey = (PathBuf, String);

pub(super) const DEFER_EXPIRY_SECS: u64 = 3 * 86_400;

#[derive(Default)]
struct RepeatState {
    counts: HashMap<String, usize>,
    deferred: HashMap<String, Deferred>,
    expired_once: HashSet<String>,
}

struct Deferred {
    expires_at: Option<u64>,
}

pub(super) struct RepeatOutcome {
    pub deferred: usize,
}

fn states() -> &'static Mutex<HashMap<RepoKey, RepeatState>> {
    static STATES: OnceLock<Mutex<HashMap<RepoKey, RepeatState>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn suppress_repeated(
    root: &Path,
    report: &mut AnalysisReport,
    limit: usize,
) -> RepeatOutcome {
    suppress_repeated_at(root, report, limit, now_secs())
}

pub(super) fn suppress_repeated_at(
    root: &Path,
    report: &mut AnalysisReport,
    limit: usize,
    now: u64,
) -> RepeatOutcome {
    let branch = crate::diff::git::current_branch(root).unwrap_or_default();
    let repo = (canon(root), branch);
    let active = active_keys(report);
    let mut states = states().lock().unwrap_or_else(|e| e.into_inner());
    let state = states.entry(repo).or_default();
    state.counts.retain(|key, _| active.contains(key));
    state.deferred.retain(|key, _| active.contains(key));
    state.expired_once.retain(|key| active.contains(key));

    let mut deferred = 0;
    retain_allowed(
        &mut report.cycles,
        state,
        limit,
        now,
        cycle_key,
        &mut deferred,
    );
    retain_allowed(
        &mut report.dead_code,
        state,
        limit,
        now,
        dead_code_key,
        &mut deferred,
    );
    retain_allowed(
        &mut report.boundaries,
        state,
        limit,
        now,
        boundary_key,
        &mut deferred,
    );
    retain_allowed(
        &mut report.duplication,
        state,
        limit,
        now,
        clone_key,
        &mut deferred,
    );
    retain_allowed(
        &mut report.smells,
        state,
        limit,
        now,
        smell_key,
        &mut deferred,
    );
    report.meta.glossary = crate::noze::glossary::for_report(report);
    RepeatOutcome { deferred }
}

fn active_keys(report: &AnalysisReport) -> HashSet<String> {
    report
        .cycles
        .iter()
        .map(cycle_key)
        .chain(report.dead_code.iter().map(dead_code_key))
        .chain(report.boundaries.iter().map(boundary_key))
        .chain(report.duplication.iter().map(clone_key))
        .chain(report.smells.iter().map(smell_key))
        .collect()
}

fn retain_allowed<T>(
    findings: &mut Vec<T>,
    state: &mut RepeatState,
    limit: usize,
    now: u64,
    key: fn(&T) -> String,
    deferred: &mut usize,
) {
    findings.retain(|finding| {
        let key = key(finding);
        if let Some(expiry) = state.deferred.get(&key).and_then(|d| d.expires_at) {
            if now >= expiry {
                state.deferred.remove(&key);
                state.expired_once.insert(key.clone());
                state.counts.insert(key.clone(), 1);
                return true;
            }
        }
        if state.deferred.contains_key(&key) {
            *deferred += 1;
            return false;
        }
        let count = state.counts.get(&key).copied().unwrap_or(0);
        if count >= limit {
            let expires_at = if state.expired_once.contains(&key) {
                None
            } else {
                Some(now.saturating_add(DEFER_EXPIRY_SECS))
            };
            state.deferred.insert(key, Deferred { expires_at });
            *deferred += 1;
            return false;
        }
        state.counts.insert(key, count + 1);
        true
    });
}

fn cycle_key(finding: &CycleFinding) -> String {
    let mut edges: Vec<String> = finding
        .edges
        .iter()
        .map(|edge| {
            format!(
                "{}:{}:{}>{}",
                path(&edge.file),
                edge.line,
                edge.from_module,
                edge.to_module
            )
        })
        .collect();
    edges.sort_unstable();
    format!("cycles|{}", edges.join("|"))
}

fn dead_code_key(finding: &DeadCodeFinding) -> String {
    format!(
        "dead_code/{}|{}:{}|{}::{}",
        finding.kind,
        path(&finding.file),
        finding.line,
        finding.module,
        finding.symbol
    )
}

fn boundary_key(finding: &BoundaryViolation) -> String {
    format!(
        "boundaries|{}:{}|{}>{}|{}",
        path(&finding.file),
        finding.line,
        finding.from_module,
        finding.to_module,
        finding.rule
    )
}

fn clone_key(finding: &CloneClass) -> String {
    let mut occurrences: Vec<String> = finding
        .occurrences
        .iter()
        .map(|o| format!("{}:{}-{}", path(&o.file), o.start_row, o.end_row))
        .collect();
    occurrences.sort_unstable();
    format!(
        "duplication|tokens:{}|{}",
        finding.token_length,
        occurrences.join("|")
    )
}

fn smell_key(finding: &SmellFinding) -> String {
    format!(
        "smells/{}|{}:{}-{}|{}",
        finding.kind,
        path(&finding.file),
        finding.line,
        finding.end_line.max(finding.line),
        finding.symbol
    )
}

fn path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn canon(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

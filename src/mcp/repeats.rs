use crate::report::{
    AnalysisReport, BoundaryViolation, CloneClass, CycleFinding, DeadCodeFinding, SmellFinding,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) const DEFER_EXPIRY_SECS: u64 = 3 * 86_400;

/// Repeat counts and auto-deferrals for one branch of one gate scope.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct RepeatState {
    counts: BTreeMap<String, usize>,
    deferred: BTreeMap<String, Deferred>,
    expired_once: BTreeSet<String>,
}

#[derive(Serialize, Deserialize)]
struct Deferred {
    expires_at: Option<u64>,
}

pub(super) struct RepeatOutcome {
    pub deferred: usize,
}

/// Drop findings reported at least `limit` times on the same lines.
pub(super) fn suppress_repeated(
    state: &mut RepeatState,
    report: &mut AnalysisReport,
    limit: usize,
    now: u64,
) -> RepeatOutcome {
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
    let span = if finding.lines.is_empty() {
        format!("{}-{}", finding.line, finding.end_line.max(finding.line))
    } else {
        let lines: Vec<String> = finding.lines.iter().map(usize::to_string).collect();
        lines.join(",")
    };
    format!(
        "smells/{}|{}:{}|{}",
        finding.kind,
        path(&finding.file),
        span,
        finding.symbol
    )
}

fn path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

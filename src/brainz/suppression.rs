//! Suppress findings that the user or repository config has already accepted.

use super::fingerprint::{self, Print};
use super::triage;
use crate::report::CloneClass;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

pub fn apply_suppressions(
    root: &Path,
    accept: &BTreeMap<String, Vec<String>>,
    report: &mut crate::report::AnalysisReport,
) {
    if let Some(patterns) = accept.get("duplication") {
        prune_accepted_occurrences(&mut report.duplication, patterns);
    }
    let triaged = triage::load(root);
    if triaged.is_empty() && accept.is_empty() {
        return;
    }
    let ignore = triage::ignored_keys(&triaged);
    let Ok(value) = serde_json::to_value(&*report) else {
        return;
    };
    let prints = fingerprint::fingerprints(&value);
    let ctx = Suppress {
        ignore: &ignore,
        accept,
    };
    fingerprint::retain_by_fingerprint(
        &mut report.cycles,
        prints.get(&fingerprint::Pillar::Cycles),
        |p| !ctx.hides(p, "cycles"),
    );
    fingerprint::retain_by_fingerprint(
        &mut report.dead_code,
        prints.get(&fingerprint::Pillar::DeadCode),
        |p| !ctx.hides(p, "dead_code"),
    );
    fingerprint::retain_by_fingerprint(
        &mut report.boundaries,
        prints.get(&fingerprint::Pillar::Boundaries),
        |p| !ctx.hides(p, "boundaries"),
    );
    fingerprint::retain_by_fingerprint(
        &mut report.duplication,
        prints.get(&fingerprint::Pillar::Duplication),
        |p| !ctx.hides(p, "duplication"),
    );
    fingerprint::retain_by_fingerprint(
        &mut report.smells,
        prints.get(&fingerprint::Pillar::Smells),
        |p| !ctx.hides(p, "smells"),
    );
}

/// Drop clone occurrences under an accepted path; `a + b` label patterns keep hiding whole groups.
fn prune_accepted_occurrences(classes: &mut Vec<CloneClass>, patterns: &[String]) {
    let paths: Vec<&str> = patterns
        .iter()
        .map(String::as_str)
        .filter(|pattern| !is_group_label(pattern))
        .collect();
    if paths.is_empty() {
        return;
    }
    classes.retain_mut(|class| {
        class.keep_occurrences(|occurrence| {
            !paths.iter().any(|path| is_under(&occurrence.file, path))
        })
    });
}

/// `a + b`: a duplication pattern naming a whole clone group; anything else is a path.
fn is_group_label(pattern: &str) -> bool {
    pattern.contains(" + ")
}

/// Whether `file` contains the whole path components of `pattern` in a row.
fn is_under(file: &Path, pattern: &str) -> bool {
    let file_parts: Vec<_> = file.components().collect();
    let pattern_parts: Vec<_> = Path::new(pattern).components().collect();
    !pattern_parts.is_empty()
        && file_parts
            .windows(pattern_parts.len())
            .any(|window| window == pattern_parts)
}

struct Suppress<'a> {
    ignore: &'a HashSet<String>,
    accept: &'a BTreeMap<String, Vec<String>>,
}

impl Suppress<'_> {
    fn hides(&self, print: &Print, pillar: &str) -> bool {
        let rendered = print.rendered();
        if self.ignore.contains(&rendered.key) {
            return true;
        }
        let matches = |key: &str| {
            self.accept.get(key).is_some_and(|pats| {
                pats.iter()
                    .filter(|p| key != "duplication" || is_group_label(p))
                    .any(|p| rendered.label.contains(p.as_str()))
            })
        };
        matches(pillar) || matches(&rendered.class)
    }
}

#[cfg(test)]
#[path = "suppression_tests.rs"]
mod tests;

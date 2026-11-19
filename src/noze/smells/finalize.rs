use crate::config::smells::{SmellConfig, Smells};
use crate::report::SmellFinding;
use crate::spine::parser::ParsedFile;
use std::collections::HashMap;

pub(super) fn apply_actions(findings: &mut [SmellFinding], cfg: &Smells) {
    for finding in findings {
        if let Some(action) = cfg.actions.get(&finding.kind) {
            finding.action = *action;
        }
    }
}

pub(super) fn apply_rule_actions(
    findings: &mut [SmellFinding],
    files: &[&ParsedFile],
    cfg: &SmellConfig,
) {
    let by_path: HashMap<_, _> = files
        .iter()
        .map(|file| (file.path.as_path(), file.language))
        .collect();
    for finding in findings {
        if let Some(language) = by_path.get(finding.file.as_path()) {
            apply_actions(std::slice::from_mut(finding), cfg.for_language(*language));
        }
    }
}

pub(super) fn fill_spans(file: &ParsedFile, out: &mut [SmellFinding]) {
    let mut span: HashMap<usize, usize> = HashMap::new();
    for f in &file.walked.units.functions {
        span.insert(f.start_line, f.end_line);
    }
    for c in &file.walked.units.classes {
        span.entry(c.start_line).or_insert(c.end_line);
    }
    for finding in out.iter_mut() {
        if finding.end_line == 0 {
            finding.end_line = span.get(&finding.line).copied().unwrap_or(finding.line);
        }
    }
}

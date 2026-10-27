//! Reader-facing hint for a clone class: size, copies, what to do, folded overlaps.

use crate::report::{CloneClass, CloneDetector, SemanticVerdict};

const DEFAULT_LABEL: &str = "structural clone";

/// `class` with a hint such as "structural clone: 73–94 tokens, 5 copies — extract a shared helper (+1 overlapping clone)".
pub(super) fn describe(mut class: CloneClass) -> CloneClass {
    let label = class.hint.as_deref().unwrap_or(DEFAULT_LABEL);
    let hint = format!(
        "{label}: {}, {} copies — {}{}",
        tokens(&class),
        class.occurrences.len(),
        advice(class.detector),
        overlaps(class.absorbed.len())
    );
    class.hint = Some(hint);
    class
}

fn advice(detector: CloneDetector) -> &'static str {
    match detector {
        CloneDetector::Semantic {
            verdict: SemanticVerdict::SameIntentDifferentCode,
        } => "keep one implementation",
        _ => "extract a shared helper",
    }
}

fn tokens(class: &CloneClass) -> String {
    let widest = class
        .absorbed
        .iter()
        .map(|absorbed| absorbed.token_length)
        .fold(class.token_length, usize::max);
    match widest > class.token_length {
        true => format!("{}–{widest} tokens", class.token_length),
        false => format!("{widest} tokens"),
    }
}

fn overlaps(count: usize) -> String {
    match count {
        0 => String::new(),
        1 => " (+1 overlapping clone)".to_string(),
        _ => format!(" (+{count} overlapping clones)"),
    }
}

#[cfg(test)]
#[path = "advice_tests.rs"]
mod tests;

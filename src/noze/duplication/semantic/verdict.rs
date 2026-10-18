use crate::config::model::SemanticDuplication;
use crate::report::{ActionLevel, SemanticVerdict};

/// Shingle Jaccard above which a copy needs only moderate code and name agreement.
const STRUCTURED_EDIT: f32 = 0.5;
const EDIT_MIN_CODE: f32 = 0.75;
const EDIT_MIN_NAMES: f32 = 0.7;
const LOOSE_EDIT_MIN_CODE: f32 = 0.85;
const LOOSE_EDIT_MIN_NAMES: f32 = 0.85;
const DOC_EDIT_MIN_CODE: f32 = 0.7;
const REWRITE_MIN_CODE: f32 = 0.8;
const REWRITE_MIN_NAMES: f32 = 0.85;

/// How alike two functions are: shingle Jaccard, and cosine of their source, signature and docs.
pub(super) struct Scores {
    pub structure: f32,
    pub code: Option<f32>,
    pub names: Option<f32>,
    pub docs: Option<f32>,
}

/// Classify a pair; `None` when the evidence is too weak to report. More structure never weakens the verdict.
pub(super) fn verdict(scores: &Scores, config: &SemanticDuplication) -> Option<SemanticVerdict> {
    let same_intent = same_intent(scores);
    if scores.structure >= min_structure(config) {
        return (edited_copy(scores, config) || same_intent).then_some(SemanticVerdict::EditedCopy);
    }
    same_intent.then_some(SemanticVerdict::SameIntentDifferentCode)
}

fn edited_copy(scores: &Scores, config: &SemanticDuplication) -> bool {
    let structured = scores.structure >= STRUCTURED_EDIT
        && at_least(scores.code, EDIT_MIN_CODE)
        && at_least(scores.names, EDIT_MIN_NAMES);
    let loose =
        at_least(scores.code, LOOSE_EDIT_MIN_CODE) && at_least(scores.names, LOOSE_EDIT_MIN_NAMES);
    let documented = at_least(scores.docs, score(config.comment_boost_score))
        && at_least(scores.code, DOC_EDIT_MIN_CODE);
    structured || loose || documented
}

fn same_intent(scores: &Scores) -> bool {
    at_least(scores.code, REWRITE_MIN_CODE) && at_least(scores.names, REWRITE_MIN_NAMES)
}

fn at_least(value: Option<f32>, floor: f32) -> bool {
    value.is_some_and(|value| value >= floor)
}

pub(super) fn action(verdict: SemanticVerdict) -> ActionLevel {
    match verdict {
        SemanticVerdict::EditedCopy => ActionLevel::Advisory,
        SemanticVerdict::SameIntentDifferentCode => ActionLevel::Info,
    }
}

pub(super) fn hint(verdict: SemanticVerdict, scores: &Scores) -> String {
    let evidence = format!(
        "structure {:.2}, code {}, names {}, docs {}",
        scores.structure,
        optional(scores.code),
        optional(scores.names),
        optional(scores.docs)
    );
    match verdict {
        SemanticVerdict::EditedCopy => format!("semantic: edited copy ({evidence})"),
        SemanticVerdict::SameIntentDifferentCode => {
            format!("semantic: same intent, structurally different code ({evidence})")
        }
    }
}

fn optional(value: Option<f32>) -> String {
    value.map_or_else(|| "n/a".to_string(), |value| format!("{value:.2}"))
}

pub(super) fn min_structure(config: &SemanticDuplication) -> f32 {
    score(config.min_shape_score)
}

fn score(score: u8) -> f32 {
    f32::from(score) / 100.0
}

#[cfg(test)]
#[path = "verdict_tests.rs"]
mod tests;

use crate::config::model::SemanticDuplication;
use crate::report::{ActionLevel, SemanticVerdict};

const UNDOCUMENTED_MIN_STRUCTURE: f32 = 0.7;
const SAME_INTENT_MIN_DOCS: f32 = 0.92;

/// Classify a pair by structure (shingle Jaccard) and doc similarity (`None` when neither has docs).
pub(super) fn verdict(
    structure: f32,
    docs: Option<f32>,
    config: &SemanticDuplication,
) -> Option<SemanticVerdict> {
    let min_structure = min_structure(config);
    match docs {
        None if structure >= min_structure && structure >= UNDOCUMENTED_MIN_STRUCTURE => {
            Some(SemanticVerdict::EditedCopy)
        }
        Some(doc) if structure >= min_structure && doc >= score(config.comment_boost_score) => {
            Some(SemanticVerdict::EditedCopy)
        }
        Some(doc) if structure < min_structure && doc >= SAME_INTENT_MIN_DOCS => {
            Some(SemanticVerdict::SameIntentDifferentCode)
        }
        _ => None,
    }
}

pub(super) fn action(verdict: SemanticVerdict) -> ActionLevel {
    match verdict {
        SemanticVerdict::EditedCopy => ActionLevel::Advisory,
        SemanticVerdict::SameIntentDifferentCode => ActionLevel::Info,
    }
}

pub(super) fn hint(verdict: SemanticVerdict, structure: f32, docs: Option<f32>) -> String {
    let docs_text = match docs {
        Some(doc) => format!("docs {doc:.2}"),
        None => "docs n/a".to_string(),
    };
    match verdict {
        SemanticVerdict::EditedCopy => {
            format!("semantic: edited copy (structure {structure:.2}, {docs_text})")
        }
        SemanticVerdict::SameIntentDifferentCode => format!(
            "semantic: near-identical docs on structurally different code (structure {structure:.2}, {docs_text})"
        ),
    }
}

pub(super) fn min_structure(config: &SemanticDuplication) -> f32 {
    score(config.min_shape_score)
}

fn score(score: u8) -> f32 {
    (score.min(100) as f32) / 100.0
}

#[cfg(test)]
#[path = "verdict_tests.rs"]
mod tests;

use super::describe;
use crate::report::{AbsorbedClone, CloneClass, CloneDetector, CloneOccurrence};

fn class(token_length: usize, copies: usize, absorbed: &[usize]) -> CloneClass {
    CloneClass {
        token_length,
        occurrences: (0..copies)
            .map(|i| CloneOccurrence {
                file: format!("f{i}.ts").into(),
                start_row: 1,
                end_row: 9,
            })
            .collect(),
        absorbed: absorbed
            .iter()
            .map(|&token_length| AbsorbedClone {
                detector: CloneDetector::Structural,
                token_length,
                copies: 2,
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn plain_clone_says_tokens_copies_and_advice() {
    assert_eq!(
        describe(class(60, 2, &[])).hint.as_deref(),
        Some("structural clone: 60 tokens, 2 copies — extract a shared helper")
    );
}

#[test]
fn folded_clones_collapse_to_a_count_and_widen_the_token_range() {
    assert_eq!(
        describe(class(195, 3, &[262, 120])).hint.as_deref(),
        Some("structural clone: 195–262 tokens, 3 copies — extract a shared helper (+2 overlapping clones)")
    );
}

#[test]
fn detector_label_is_kept() {
    let mut renamed = class(80, 2, &[]);
    renamed.hint = Some("consistent-rename clone".into());
    assert_eq!(
        describe(renamed).hint.as_deref(),
        Some("consistent-rename clone: 80 tokens, 2 copies — extract a shared helper")
    );
}

#[test]
fn same_intent_rewrite_is_told_to_keep_one_implementation() {
    let mut rewrite = class(70, 2, &[]);
    rewrite.detector = CloneDetector::Semantic {
        verdict: crate::report::SemanticVerdict::SameIntentDifferentCode,
    };
    assert!(rewrite_hint(rewrite).ends_with("— keep one implementation"));
}

fn rewrite_hint(class: CloneClass) -> String {
    describe(class).hint.unwrap_or_default()
}

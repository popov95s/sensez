use super::*;

fn config() -> SemanticDuplication {
    SemanticDuplication {
        enabled: true,
        ..SemanticDuplication::default()
    }
}

#[test]
fn edited_copy_needs_structure_and_matching_docs() {
    assert_eq!(
        verdict(0.76, Some(0.91), &config()),
        Some(SemanticVerdict::EditedCopy)
    );
    assert_eq!(verdict(0.76, Some(0.70), &config()), None);
    assert_eq!(verdict(0.24, Some(0.90), &config()), None);
}

#[test]
fn undocumented_pair_needs_high_structure() {
    assert_eq!(verdict(0.6, None, &config()), None);
    assert_eq!(
        verdict(0.7, None, &config()),
        Some(SemanticVerdict::EditedCopy)
    );
}

#[test]
fn near_identical_docs_on_different_code_is_same_intent() {
    assert_eq!(
        verdict(0.01, Some(0.95), &config()),
        Some(SemanticVerdict::SameIntentDifferentCode)
    );
    assert_eq!(
        action(SemanticVerdict::SameIntentDifferentCode),
        ActionLevel::Info
    );
    assert_eq!(action(SemanticVerdict::EditedCopy), ActionLevel::Advisory);
}

#[test]
fn hints_name_the_verdict_and_scores() {
    assert_eq!(
        hint(SemanticVerdict::EditedCopy, 0.48, Some(0.91)),
        "semantic: edited copy (structure 0.48, docs 0.91)"
    );
    assert_eq!(
        hint(SemanticVerdict::EditedCopy, 0.72, None),
        "semantic: edited copy (structure 0.72, docs n/a)"
    );
    assert_eq!(
        hint(SemanticVerdict::SameIntentDifferentCode, 0.01, Some(0.95)),
        "semantic: near-identical docs on structurally different code (structure 0.01, docs 0.95)"
    );
}

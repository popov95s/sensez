use super::*;

fn config() -> SemanticDuplication {
    SemanticDuplication {
        enabled: true,
        ..SemanticDuplication::default()
    }
}

fn scores(structure: f32, code: f32, names: f32, docs: Option<f32>) -> Scores {
    Scores {
        structure,
        code: Some(code),
        names: Some(names),
        docs,
    }
}

#[test]
fn copy_with_inserted_statements_is_an_edited_copy() {
    assert_eq!(
        verdict(&scores(0.85, 0.98, 1.0, None), &config()),
        Some(SemanticVerdict::EditedCopy)
    );
}

#[test]
fn loosely_structured_copy_needs_strong_code_and_name_agreement() {
    assert_eq!(
        verdict(&scores(0.41, 0.91, 1.0, None), &config()),
        Some(SemanticVerdict::EditedCopy)
    );
    assert_eq!(verdict(&scores(0.41, 0.70, 0.70, None), &config()), None);
}

#[test]
fn same_shape_with_different_names_is_not_a_copy() {
    assert_eq!(verdict(&scores(0.71, 0.89, 0.25, None), &config()), None);
}

#[test]
fn matching_docs_confirm_an_edited_copy() {
    assert_eq!(
        verdict(&scores(0.62, 0.75, 0.3, Some(0.95)), &config()),
        Some(SemanticVerdict::EditedCopy)
    );
}

#[test]
fn shared_doc_on_unrelated_code_is_not_a_copy() {
    assert_eq!(
        verdict(&scores(0.62, 0.4, 0.3, Some(0.95)), &config()),
        None
    );
}

#[test]
fn rewrite_needs_similar_source_and_signature() {
    assert_eq!(
        verdict(&scores(0.03, 0.88, 0.97, None), &config()),
        Some(SemanticVerdict::SameIntentDifferentCode)
    );
    assert_eq!(verdict(&scores(0.03, 0.88, 0.5, None), &config()), None);
    assert_eq!(verdict(&scores(0.03, 0.6, 0.97, None), &config()), None);
}

#[test]
fn rewrites_are_info_and_copies_are_advisory() {
    assert_eq!(
        action(SemanticVerdict::SameIntentDifferentCode),
        ActionLevel::Info
    );
    assert_eq!(action(SemanticVerdict::EditedCopy), ActionLevel::Advisory);
}

#[test]
fn hints_name_the_verdict_and_every_score() {
    assert_eq!(
        hint(SemanticVerdict::EditedCopy, &scores(0.48, 0.91, 0.88, None)),
        "semantic: edited copy (structure 0.48, code 0.91, names 0.88, docs n/a)"
    );
    assert_eq!(
        hint(
            SemanticVerdict::SameIntentDifferentCode,
            &scores(0.12, 0.86, 0.92, Some(0.4))
        ),
        "semantic: same intent, structurally different code (structure 0.12, code 0.86, names 0.92, docs 0.40)"
    );
}

#[test]
fn more_structure_never_drops_a_rewrite_finding() {
    let rewrite = scores(0.1, 0.82, 0.9, None);
    let more_structure = scores(0.4, 0.82, 0.9, None);
    assert!(verdict(&rewrite, &config()).is_some());
    assert_eq!(
        verdict(&more_structure, &config()),
        Some(SemanticVerdict::EditedCopy)
    );
}

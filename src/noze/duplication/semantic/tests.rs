use super::*;

#[test]
fn repeated_method_name_uses_nearest_documentation() {
    let comments = FxHashMap::from_iter([
        (
            "m::First.run".to_string(),
            (
                2,
                DocKind::Comment,
                "Run the first scheduled job safely".to_string(),
            ),
        ),
        (
            "m::Second.run".to_string(),
            (
                21,
                DocKind::Comment,
                "Run the second scheduled job safely".to_string(),
            ),
        ),
    ]);
    let func = FunctionUnit {
        name: "run".into(),
        start_line: 22,
        end_line: 29,
        ..Default::default()
    };
    assert_eq!(
        comment_for(&comments, &func, true).unwrap().0,
        "m::Second.run"
    );
}

#[test]
fn implementation_comment_is_not_function_documentation() {
    let comments = FxHashMap::from_iter([(
        "m::run".to_string(),
        (
            23,
            DocKind::Comment,
            "This comment describes an internal branch".to_string(),
        ),
    )]);
    let func = FunctionUnit {
        name: "run".into(),
        start_line: 20,
        end_line: 29,
        ..Default::default()
    };
    assert!(comment_for(&comments, &func, true).is_none());
}

#[test]
fn regression_documented_functions_form_a_candidate_without_internal_comment_noise() {
    use crate::spine::parser::parse_file;

    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("regression/fixtures/eyez");
    let files: Vec<_> = [
        "documented_a.js",
        "documented_b.js",
        "internal_comment_a.js",
        "internal_comment_b.js",
    ]
    .iter()
    .enumerate()
    .map(|(index, name)| parse_file(&fixture_root.join(name), index as u32).unwrap())
    .collect();
    let refs: Vec<_> = files.iter().collect();
    let units = collect_units(&refs, true);
    assert_eq!(units.len(), 2, "only leading function docs should qualify");
    assert_eq!(candidate_pairs(&units, 80).len(), 1);
    assert!(units.iter().all(|unit| unit.comment.contains("transport")));
}

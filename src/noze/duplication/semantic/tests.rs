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

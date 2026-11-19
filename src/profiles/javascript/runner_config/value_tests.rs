use super::*;

fn object(entries: Vec<(&str, Value)>) -> Value {
    Value::Object(Object {
        entries: entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
        unknown: None,
    })
}

#[test]
fn merge_concatenates_arrays_and_merges_objects() {
    let base = object(vec![(
        "test",
        object(vec![(
            "setupFiles",
            Value::Array(vec![Value::Str("a".into())]),
        )]),
    )]);
    let top = object(vec![(
        "test",
        object(vec![
            ("setupFiles", Value::Array(vec![Value::Str("b".into())])),
            ("include", Value::Str("x".into())),
        ]),
    )]);

    let merged = merge(base, top);
    let test = merged.get("test").unwrap();

    assert_eq!(
        test.get("setupFiles").unwrap().strings().unwrap(),
        ["a", "b"]
    );
    assert_eq!(test.get("include").unwrap().strings().unwrap(), ["x"]);
}

#[test]
fn unknown_spread_makes_missing_keys_opaque() {
    let value = Value::Object(Object {
        entries: Vec::new(),
        unknown: Some("spread of `shared()`".into()),
    });

    assert!(matches!(value.get("include"), Some(Value::Opaque(_))));
}

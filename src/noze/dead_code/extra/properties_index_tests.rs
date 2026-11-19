use super::*;

#[test]
fn type_parts_extracts_identifier_segments() {
    let found: Vec<_> = type_parts(
        "typing.Optional['pkg.models.Foo'] | crate::domain::Thing | React.Component<Props>",
    )
    .collect();

    assert_eq!(
        found,
        vec![
            "Optional",
            "typing",
            "Foo",
            "models",
            "pkg",
            "crate",
            "domain",
            "Thing",
            "Component",
            "React",
            "Props"
        ]
    );
}

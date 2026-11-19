use super::*;

#[test]
fn arrow_inside_generic_does_not_close_a_bracket() {
    assert_eq!(
        split_top_level("Foo<A<() => B, C>>, D", ','),
        vec!["Foo<A<() => B, C>>", "D"]
    );
}

#[test]
fn union_inside_generic_stays_nominal() {
    assert!(is_nominal_ignoring("Repo<A | B> | null", &["null"], &[]));
    assert!(!is_nominal("A | B", &[]));
}

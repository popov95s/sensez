use super::*;

#[test]
fn position_is_one_indexed() {
    assert_eq!(position("a = 1\nbb = 2\n", 6), (2, 1));
    assert_eq!(position("a = 1\nbb = 2\n", 11), (2, 6));
}

#[test]
fn unknown_variant_gets_a_suggestion() {
    let message =
        "unknown variant `must_fx`, expected one of `info`, `advisory`, `warning`, `must_fix`";
    assert!(with_variant_suggestion(message).ends_with("(did you mean `must_fix`?)"));
    assert_eq!(
        with_variant_suggestion("unknown variant `critical`, expected `a` or `b`"),
        "unknown variant `critical`, expected `a` or `b`"
    );
}

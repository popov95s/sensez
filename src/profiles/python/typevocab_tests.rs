use super::*;

#[test]
fn qualified_optional_is_unwrapped_before_the_nominal_check() {
    assert!(is_nominal_type("typing.Optional[Order]"));
    assert!(!is_nominal_type("typing.Optional[Order | Invoice]"));
}

#[test]
fn sets_of_primitives_are_loose_but_sets_of_domain_types_are_not() {
    assert!(is_loose("set[int]"));
    assert!(is_loose("set[str]"));
    assert!(is_loose("Set[int]"));
    assert!(is_loose("frozenset[str]"));
    assert!(is_loose("Optional[set[int]]"));
    assert!(!is_loose("set[UserModel]"));
    // Parity with the existing containers stays intact.
    assert!(is_loose("list[int]"));
    assert!(!is_loose("list[UserModel]"));
    assert!(!is_loose("Literal[\"json\", \"xml\"]"));
}

#[test]
fn bool_and_dictish() {
    assert!(is_bool("bool"));
    assert!(!is_bool("boolean"));
    assert!(is_dictish("dict[str, int]"));
    assert!(!is_dictish("DataFrame"));
}

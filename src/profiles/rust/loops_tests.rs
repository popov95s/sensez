use super::trip_count;

#[test]
fn exclusive_range_subtracts_its_start() {
    assert_eq!(trip_count("5..8"), Some(3));
}

#[test]
fn inclusive_range_counts_its_end() {
    assert_eq!(trip_count("0..=3"), Some(4));
}

#[test]
fn take_counts_its_argument() {
    assert_eq!(trip_count("items.iter().take(7)"), Some(7));
}

#[test]
fn non_literal_range_has_no_count() {
    assert_eq!(trip_count("0..n"), None);
}

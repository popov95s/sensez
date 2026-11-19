use super::trip_count;

#[test]
fn stop_only_range_counts_stop() {
    assert_eq!(trip_count("range(3)"), Some(3));
}

#[test]
fn start_stop_range_subtracts_start() {
    assert_eq!(trip_count("range(1000,1003)"), Some(3));
}

#[test]
fn stepped_range_divides_by_step() {
    assert_eq!(trip_count("range(0,10,3)"), Some(4));
}

#[test]
fn non_literal_range_has_no_count() {
    assert_eq!(trip_count("range(n)"), None);
}

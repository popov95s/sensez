use super::index_bound;
use crate::spine::ir::LoopBound;

#[test]
fn inclusive_bound_at_usize_max_saturates() {
    let bound = index_bound("i <= 18446744073709551615", 0);
    assert_eq!(bound, LoopBound::Count(usize::MAX));
}

#[test]
fn index_bound_subtracts_literal_start() {
    assert_eq!(index_bound("i < 8", 5), LoopBound::Count(3));
}

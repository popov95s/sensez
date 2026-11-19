use super::*;

#[test]
fn reset_discards_prior_measurements() {
    READ_US.store(10, Ordering::Relaxed);
    reset();
    assert_eq!(take().read, Duration::ZERO);
}

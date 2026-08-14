//! Opt-in aggregate timings for the parallel source-fact pipeline.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

static READ_US: AtomicU64 = AtomicU64::new(0);
static HASH_US: AtomicU64 = AtomicU64::new(0);
static TREE_US: AtomicU64 = AtomicU64::new(0);
static WALK_US: AtomicU64 = AtomicU64::new(0);
static DEPTH_US: AtomicU64 = AtomicU64::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub(crate) struct Breakdown {
    pub read: Duration,
    pub hash: Duration,
    pub tree: Duration,
    pub walk: Duration,
    pub depth: Duration,
}

pub(crate) fn reset() {
    ENABLED.store(
        std::env::var_os("SENSEZ_TIMING").is_some(),
        Ordering::Relaxed,
    );
    for value in [&READ_US, &HASH_US, &TREE_US, &WALK_US, &DEPTH_US] {
        value.store(0, Ordering::Relaxed);
    }
}

pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub(crate) fn record_read(duration: Duration) {
    add(&READ_US, duration);
}

pub(crate) fn record_hash(duration: Duration) {
    add(&HASH_US, duration);
}

pub(crate) fn record_tree(duration: Duration) {
    add(&TREE_US, duration);
}

pub(crate) fn record_walk(duration: Duration) {
    add(&WALK_US, duration);
}

pub(crate) fn record_depth(duration: Duration) {
    add(&DEPTH_US, duration);
}

pub(crate) fn take() -> Breakdown {
    Breakdown {
        read: take_duration(&READ_US),
        hash: take_duration(&HASH_US),
        tree: take_duration(&TREE_US),
        walk: take_duration(&WALK_US),
        depth: take_duration(&DEPTH_US),
    }
}

fn add(total: &AtomicU64, duration: Duration) {
    total.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
}

fn take_duration(total: &AtomicU64) -> Duration {
    Duration::from_micros(total.swap(0, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_discards_prior_measurements() {
        READ_US.store(10, Ordering::Relaxed);
        reset();
        assert_eq!(take().read, Duration::ZERO);
    }
}

//! Persistence flush for buffered metrics events.

use super::{file_lock, hub, store};

const EVENTS_RETENTION_SECS: u64 = 45 * 86_400;

pub fn flush() {
    for (root, batch) in hub::drain_pending() {
        // Acquire exclusive lock for the entire flush sequence to prevent
        // cross-process interleaving of read-modify-write operations.
        let _lock = match file_lock::acquire(&root, "flush.lock") {
            Ok(lock) => lock,
            Err(err) => {
                eprintln!("[sensez metrics] acquiring flush lock: {err:#}");
                continue;
            }
        };

        let mut totals = store::load_totals(&root);
        for event in &batch {
            totals.absorb(event);
        }
        if let Err(err) = store::append_events(&root, &batch) {
            eprintln!("[sensez metrics] appending events: {err:#}");
        }
        if let Err(err) = store::save_totals(&root, &totals) {
            eprintln!("[sensez metrics] saving totals: {err:#}");
        }
        if let Err(err) =
            store::compact_events(&root, hub::now().saturating_sub(EVENTS_RETENTION_SECS))
        {
            eprintln!("[sensez metrics] compacting events: {err:#}");
        }
    }
}

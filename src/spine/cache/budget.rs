pub(super) const TOTAL_BYTES: usize = 1_000_000;

pub(super) fn remove_oversized(path: &std::path::Path, limit: usize) {
    if std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > limit as u64) {
        let _ = std::fs::remove_file(path);
    }
}

pub(super) const TOTAL_BYTES: usize = 1_000_000;

pub(super) fn remove_oversized(path: &std::path::Path, limit: usize) {
    if std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > limit as u64) {
        let _ = std::fs::remove_file(path);
    }
}

pub(super) fn enforce_total(root: &std::path::Path) {
    let snapshot = root.join(".sensez/analysis-v1.bin");
    let parsed = root.join(".sensez/parse-v2.bin");
    remove_oversized(&snapshot, TOTAL_BYTES);
    remove_oversized(&parsed, TOTAL_BYTES);
    let size = |path: &std::path::Path| {
        std::fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or_default()
    };
    if size(&snapshot) + size(&parsed) > TOTAL_BYTES as u64 {
        let _ = std::fs::remove_file(parsed);
    }
}

use crate::report::CloneOccurrence;
use crate::spine::ir::FileId;
use crate::spine::parser::tokens::TokenSpan;
use crate::spine::parser::ParsedFile;

/// Map a master-buffer token range `[start, end)` to a concrete file/line span.
pub(super) fn occurrence(
    files: &[&ParsedFile],
    spans: &[TokenSpan],
    start: usize,
    end: usize,
) -> Option<CloneOccurrence> {
    let first = spans.get(start)?;
    let last = spans.get(end.saturating_sub(1))?;
    if first.file_id == FileId::SEPARATOR || last.file_id == FileId::SEPARATOR {
        return None;
    }
    let file = files.get(first.file_id.index())?.path.clone();
    Some(CloneOccurrence {
        file,
        start_row: first.start_row as usize,
        end_row: last_closed_row(spans, start, end).unwrap_or(last.start_row) as usize,
    })
}

fn last_closed_row(spans: &[TokenSpan], start: usize, end: usize) -> Option<u32> {
    (start..end)
        .filter(|&t| closes_within(spans, start, end, t))
        .map(|t| spans[t].end_row)
        .max()
}

/// Whether token `t`'s node ends before the token after match `[start, end)` in the same file.
fn closes_within(spans: &[TokenSpan], start: usize, end: usize, t: usize) -> bool {
    match spans.get(end) {
        Some(next) if next.file_id == spans[start].file_id => spans[t].end_byte <= next.start_byte,
        _ => true,
    }
}

/// Drop occurrences that overlap an already-kept one in the same file.
pub(super) fn suppress_overlaps(mut occ: Vec<CloneOccurrence>) -> Vec<CloneOccurrence> {
    occ.sort_by(|a, b| (a.file.as_path(), a.start_row).cmp(&(b.file.as_path(), b.start_row)));
    let mut kept: Vec<CloneOccurrence> = Vec::new();
    for o in occ {
        let overlaps = kept
            .last()
            .is_some_and(|k| k.file == o.file && o.start_row <= k.end_row);
        if !overlaps {
            kept.push(o);
        }
    }
    kept
}

/// Order-independent key so the same clone class is reported once.
pub(super) fn dedup_key(occ: &[CloneOccurrence], len: usize) -> u64 {
    use std::hash::{Hash, Hasher};
    let combined = occ.iter().fold(0u64, |acc, o| {
        let mut h = rustc_hash::FxHasher::default();
        (o.file.as_path(), o.start_row).hash(&mut h);
        acc ^ h.finish()
    });
    combined.rotate_left(1) ^ (len as u64)
}

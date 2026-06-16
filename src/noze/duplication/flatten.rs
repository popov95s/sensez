//! Flatten all files into one master buffer of lexeme codes.
//!
//! Layout: `[file0 lexemes][sep0][file1 lexemes][sep1]...[term]`. Separators
//! are globally unique and live in a high namespace that lexeme codes never
//! reach, so no shared prefix between two suffixes can span a separator —
//! cross-file matches across a boundary are impossible by construction. Codes
//! are then densified to a contiguous `0..K` alphabet for `suffix_array_int`.

use crate::spine::parser::tokens::TokenSpan;
use crate::spine::parser::ParsedFile;
use rustc_hash::FxHashMap;

/// Sentinel `file_id` marking a separator/terminator position (not a token).
pub const NON_TOKEN: u32 = u32::MAX;

/// Per-file separator namespace, above every lexeme code (see `lexeme.rs`).
const SEP_BASE: u64 = 1 << 62;

/// The flattened master buffer plus a per-position source span.
pub struct Master {
    pub text: Vec<usize>,
    pub spans: Vec<TokenSpan>,
}

/// Build the densified master buffer over `files` (file ids are local indices
/// into this slice, so callers may pass a filtered subset).
pub fn build(files: &[&ParsedFile]) -> Master {
    let mut raw: Vec<u64> = Vec::new();
    let mut spans: Vec<TokenSpan> = Vec::new();

    for (file_idx, file) in files.iter().enumerate() {
        for (code, span) in file
            .walked
            .syntax
            .lexemes
            .iter()
            .zip(&file.walked.syntax.spans)
        {
            raw.push(*code);
            spans.push(TokenSpan {
                file_id: file_idx as u32,
                start_row: span.start_row,
                end_row: span.end_row,
            });
        }
        raw.push(SEP_BASE + file_idx as u64);
        spans.push(non_token_span());
    }
    raw.push(0); // global terminator (lexeme codes are all >= 1)
    spans.push(non_token_span());

    Master {
        text: densify(&raw),
        spans,
    }
}

fn non_token_span() -> TokenSpan {
    TokenSpan {
        file_id: NON_TOKEN,
        start_row: 0,
        end_row: 0,
    }
}

/// Remap raw codes to a contiguous `0..K` alphabet for `suffix_array_int`.
///
/// bio imposes exactly two constraints: the alphabet must be gap-free `0..K`
/// (it indexes by symbol, so a missing value panics), and the sentinel must be
/// the lexicographically smallest symbol. Beyond that the *order* of symbols is
/// irrelevant here: clone groups are maximal runs of suffixes sharing a prefix
/// of equal symbols, and that equivalence — hence every emitted clone — is
/// invariant under any bijective relabeling. A first-seen pass over a flat hash
/// map yields dense ids in O(n); the global terminator (raw `0`, the unique
/// minimum) is pinned to id `0` so it remains the smallest symbol.
fn densify(raw: &[u64]) -> Vec<usize> {
    let mut id_of: FxHashMap<u64, usize> = FxHashMap::default();
    id_of.insert(0, 0); // terminator stays the lexicographically smallest symbol
    let mut next = 1usize;
    raw.iter()
        .map(|&value| match id_of.get(&value) {
            Some(&id) => id,
            None => {
                let id = next;
                next += 1;
                id_of.insert(value, id);
                id
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `densify` must yield a gap-free `0..K` alphabet with the terminator (raw
    /// `0`) pinned to id `0` and a consistent bijection (equal raws → equal ids,
    /// distinct raws → distinct ids). These are exactly bio's `suffix_array_int`
    /// preconditions; if this regresses, the SA panics or mis-groups clones.
    #[test]
    fn densify_is_contiguous_bijection_with_zero_sentinel() {
        // Terminator 0 last; high separator values interleaved with real codes.
        let raw: Vec<u64> = vec![50, 17, 50, 1 << 62, 17, 99, 50, 0];
        let dense = densify(&raw);

        // Terminator → smallest id.
        assert_eq!(dense[raw.len() - 1], 0, "sentinel must be the smallest id");

        // Contiguous 0..K: the set of ids is exactly {0, 1, ..., K-1}.
        let mut ids: Vec<usize> = dense.clone();
        ids.sort_unstable();
        ids.dedup();
        let k = ids.len();
        assert_eq!(ids, (0..k).collect::<Vec<_>>(), "alphabet must be gap-free");

        // Bijection: positions are equal in `dense` iff equal in `raw`.
        for i in 0..raw.len() {
            for j in 0..raw.len() {
                assert_eq!(
                    raw[i] == raw[j],
                    dense[i] == dense[j],
                    "relabeling must preserve the equality structure"
                );
            }
        }
    }
}

//! Flatten all files into one master buffer of lexeme codes.

use crate::spine::ir::FileId;
use crate::spine::parser::tokens::TokenSpan;
use crate::spine::parser::ParsedFile;
use rustc_hash::FxHashMap;

/// Per-file separator namespace, above every lexeme code (see `lexeme.rs`).
const SEP_BASE: u64 = 1 << 62;

/// The flattened master buffer plus a per-position source span.
pub struct Master {
    pub text: Vec<i32>,
    pub spans: Vec<TokenSpan>,
}

pub fn build(files: &[&ParsedFile]) -> Master {
    let capacity = files
        .iter()
        .map(|file| file.walked.syntax.lexemes.len() + 1)
        .sum::<usize>()
        + 1;
    let mut text = Vec::with_capacity(capacity);
    let mut spans = Vec::with_capacity(capacity);
    let mut dense = DenseAlphabet::new();

    for (file_idx, file) in files.iter().enumerate() {
        for (code, span) in file
            .walked
            .syntax
            .lexemes
            .iter()
            .zip(&file.walked.syntax.spans)
        {
            text.push(dense.id(*code));
            spans.push(TokenSpan {
                file_id: FileId::from_index(file_idx),
                ..*span
            });
        }
        text.push(dense.id(SEP_BASE + file_idx as u64));
        spans.push(non_token_span());
    }
    text.push(0); // global terminator (lexeme codes are all >= 1)
    spans.push(non_token_span());

    Master { text, spans }
}

fn non_token_span() -> TokenSpan {
    TokenSpan {
        file_id: FileId::SEPARATOR,
        start_row: 0,
        end_row: 0,
        start_byte: 0,
        end_byte: 0,
    }
}

/// Remap raw codes to a contiguous `0..K` alphabet for `suffix_array_int`.
struct DenseAlphabet {
    id_of: FxHashMap<u64, i32>,
    next: i32,
}

impl DenseAlphabet {
    fn new() -> Self {
        Self {
            id_of: FxHashMap::from_iter([(0, 0)]),
            next: 1,
        }
    }

    fn id(&mut self, value: u64) -> i32 {
        match self.id_of.get(&value) {
            Some(&id) => id,
            None => {
                let id = self.next;
                self.next += 1;
                self.id_of.insert(value, id);
                id
            }
        }
    }
}

#[cfg(test)]
fn densify(raw: &[u64]) -> Vec<i32> {
    let mut alphabet = DenseAlphabet::new();
    raw.iter().map(|&value| alphabet.id(value)).collect()
}

#[cfg(test)]
#[path = "flatten_tests.rs"]
mod tests;

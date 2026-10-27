//! Per-function lexeme slices and their edit-distance similarity.

use crate::spine::parser::{FunctionUnit, ParsedFile};

/// Lexeme codes of one function's source lines (local names already collapsed).
pub(crate) fn function_lexemes<'a>(file: &'a ParsedFile, func: &FunctionUnit) -> &'a [u64] {
    &file.walked.syntax.lexemes[super::span_index_range(file, func.start_line, func.end_line)]
}

/// Percent of `left` and `right` left unchanged by their lexeme edit distance.
pub(crate) fn similarity_percent(left: &[u64], right: &[u64]) -> usize {
    let longest = left.len().max(right.len());
    if longest == 0 {
        return 100;
    }
    100 - strsim::generic_levenshtein(&left.to_vec(), &right.to_vec()) * 100 / longest
}

/// Upper bound of [`similarity_percent`] from the lengths alone.
pub(crate) fn length_ceiling_percent(left: &[u64], right: &[u64]) -> usize {
    let longest = left.len().max(right.len()).max(1);
    left.len().min(right.len()) * 100 / longest
}

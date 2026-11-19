use crate::spine::parser::tokens::TokenSpan;
use crate::spine::parser::{FunctionUnit, ParsedFile};

pub(in crate::noze) fn top_level_functions(file: &ParsedFile) -> Vec<&FunctionUnit> {
    file.walked
        .units
        .functions
        .iter()
        .filter(|func| !func.is_nested)
        .collect()
}

pub(in crate::noze) fn span_index_range(
    file: &ParsedFile,
    start_line: usize,
    end_line: usize,
) -> std::ops::Range<usize> {
    let spans = &file.walked.syntax.spans;
    let row = |span: &TokenSpan| span.start_row as usize;
    let lo = spans.partition_point(|span| row(span) < start_line);
    let hi = spans.partition_point(|span| row(span) <= end_line);
    lo..hi.max(lo)
}

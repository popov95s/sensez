//! Shared comment-to-function attachment for smells.

use crate::spine::ir::Walked;
use std::cmp::Reverse;

/// Attach each recorded comment span to the innermost function that contains
/// it. Nested functions keep their own comments; outer functions do not inherit
/// them.
pub(crate) fn attach(out: &mut Walked) {
    if out.units.comment_spans.is_empty() || out.units.functions.is_empty() {
        return;
    }

    let mut functions: Vec<usize> = (0..out.units.functions.len()).collect();
    functions.sort_by_key(|&i| {
        let f = &out.units.functions[i];
        (f.start_line, Reverse(f.end_line))
    });

    let mut comments: Vec<usize> = (0..out.units.comment_spans.len()).collect();
    comments.sort_by_key(|&i| out.units.comment_spans[i].start_line);

    let mut counts = vec![0usize; out.units.functions.len()];
    let mut active: Vec<usize> = Vec::new();
    let mut next_function = 0;

    for comment_index in comments {
        let comment = &out.units.comment_spans[comment_index];
        while next_function < functions.len()
            && out.units.functions[functions[next_function]].start_line <= comment.start_line
        {
            active.push(functions[next_function]);
            next_function += 1;
        }
        active.retain(|&i| comment.end_line <= out.units.functions[i].end_line);

        if let Some(index) = active
            .iter()
            .copied()
            .max_by_key(|&i| out.units.functions[i].start_line)
        {
            counts[index] += comment.end_line - comment.start_line + 1;
        }
    }

    for (function, count) in out.units.functions.iter_mut().zip(counts) {
        function.comment_lines = count;
    }
}

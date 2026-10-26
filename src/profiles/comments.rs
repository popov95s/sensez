//! Shared comment-to-function attachment for smells.

use crate::spine::ir::{CommentClass, CommentSpan, FunctionUnit, Walked};
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

        let owner = active
            .iter()
            .copied()
            .max_by_key(|&i| out.units.functions[i].start_line);
        if let Some(index) = owner {
            credit(&mut out.units.functions[index], comment);
        }
    }
}

fn credit(function: &mut FunctionUnit, comment: &CommentSpan) {
    let lines = comment.end_line - comment.start_line + 1;
    function.comment_lines += lines;
    match comment.class {
        CommentClass::Narration => function.narration_comment_lines += lines,
        CommentClass::Code => function.code_comment_lines += lines,
        CommentClass::Exempt => return,
    }
    function.first_noisy_comment_line = Some(
        function
            .first_noisy_comment_line
            .map_or(comment.start_line, |line| line.min(comment.start_line)),
    );
}

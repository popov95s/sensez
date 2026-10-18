use crate::profiles::comment_class::CommentSyntax;

pub(crate) const COMMENT_SYNTAX: CommentSyntax = CommentSyntax {
    markers: &["#"],
    closers: &[],
    directives: &["noqa", "type:", "pragma", "pylint:", "fmt:"],
    markup_parents: &[],
    code_endings: &[')', ']'],
    code_fragments: &[" = "],
};

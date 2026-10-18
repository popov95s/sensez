use crate::profiles::comment_class::CommentSyntax;

pub(crate) const COMMENT_SYNTAX: CommentSyntax = CommentSyntax {
    markers: &["//!", "///", "/*!", "/**", "/*", "//", "*"],
    closers: &["*/"],
    directives: &["rustfmt::skip"],
    markup_parents: &[],
    code_endings: &[';', '{', '}', ')'],
    code_fragments: &["=>"],
};

use crate::profiles::comment_class::CommentSyntax;

pub(crate) const COMMENT_SYNTAX: CommentSyntax = CommentSyntax {
    markers: &["/**", "/*", "//", "*"],
    closers: &["*/"],
    directives: &[
        "eslint",
        "oxlint",
        "biome-ignore",
        "@ts-",
        "prettier-ignore",
        "istanbul",
        "c8",
        "v8 ignore",
        "tslint",
        "jshint",
        "global",
        "#region",
        "#endregion",
    ],
    markup_parents: &["jsx_expression"],
    code_endings: &[';', '{', '}', ')'],
    code_fragments: &["=>"],
};

use super::{has_word, is_directive, CommentSyntax};

const SYNTAX: CommentSyntax = CommentSyntax {
    markers: &["//"],
    closers: &[],
    directives: &["global", "@ts-"],
    markup_parents: &[],
    code_endings: &[],
    code_fragments: &[],
};

#[test]
fn directive_must_end_on_a_word_boundary() {
    assert!(is_directive("global window", &SYNTAX));
    assert!(is_directive("@ts-expect-error", &SYNTAX));
    assert!(!is_directive("globally cached for speed", &SYNTAX));
}

#[test]
fn marker_must_end_on_a_word_boundary() {
    assert!(has_word("note: keep sorted", "note"));
    assert!(!has_word("open the notebook", "note"));
    assert!(!has_word("sincerely yours", "since"));
}

#[test]
fn marker_ending_in_punctuation_matches_a_prefix() {
    assert!(has_word("see docs/limits.md", "see "));
}

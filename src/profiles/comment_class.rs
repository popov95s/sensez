//! Classify a source comment as directive, markup, code, narration, or rationale.

use crate::profiles::node_query::node_text;
use crate::spine::ir::CommentClass;
use tree_sitter::Node;

/// Per-language comment shapes.
pub(crate) struct CommentSyntax {
    /// Comment openers stripped before reading the text, longest first.
    pub(crate) markers: &'static [&'static str],
    pub(crate) closers: &'static [&'static str],
    /// Tool directive prefixes (`eslint-disable`, `noqa`).
    pub(crate) directives: &'static [&'static str],
    /// Parent kinds that make a comment a markup label (`{/* … */}` in JSX).
    pub(crate) markup_parents: &'static [&'static str],
    pub(crate) code_endings: &'static [char],
    pub(crate) code_fragments: &'static [&'static str],
}

const RATIONALE_MARKERS: &[&str] = &[
    "because",
    "since",
    "so that",
    "otherwise",
    "instead",
    "workaround",
    "hack",
    "note",
    "why",
    "must",
    "never",
    "always",
    "do not",
    "don't",
    "avoid",
    "intentional",
    "intentionally",
    "deliberate",
    "deliberately",
    "upstream",
    "limit",
    "limits",
    "limitation",
    "incident",
    "compat",
    "compatible",
    "compatibility",
    "see ",
    "http://",
    "https://",
];
const RATIONALE_MIN_WORDS: usize = 12;

pub(crate) fn classify(
    node: Node,
    parent: Option<Node>,
    src: &[u8],
    syntax: &CommentSyntax,
) -> CommentClass {
    let lines = body_lines(node, src, syntax);
    if lines.first().is_some_and(|line| is_directive(line, syntax)) {
        return CommentClass::Directive;
    }
    if parent.is_some_and(|p| syntax.markup_parents.contains(&p.kind())) {
        return CommentClass::Markup;
    }
    let block: Vec<&str> = block_nodes(node, parent)
        .into_iter()
        .flat_map(|comment| body_lines(comment, src, syntax))
        .collect();
    let is_code = |line: &&str| looks_like_code(line, syntax);
    if block.iter().filter(|line| is_code(line)).count() >= 2
        && !lines.is_empty()
        && lines.iter().all(is_code)
    {
        return CommentClass::Code;
    }
    if has_rationale(&block.join(" ")) {
        return CommentClass::Rationale;
    }
    CommentClass::Narration
}

/// `node` plus the same-kind comments on directly adjacent lines.
fn block_nodes<'t>(node: Node<'t>, parent: Option<Node<'t>>) -> Vec<Node<'t>> {
    let Some(parent) = parent else {
        return vec![node];
    };
    let adjacent = |upper: &Node, lower: &Node| {
        upper.kind() == lower.kind() && upper.end_position().row + 1 >= lower.start_position().row
    };
    let mut cursor = parent.walk();
    let mut block: Vec<Node> = Vec::new();
    for sibling in parent.named_children(&mut cursor) {
        if block.last().is_some_and(|last| !adjacent(last, &sibling)) {
            if block.iter().any(|member| member.id() == node.id()) {
                break;
            }
            block.clear();
        }
        block.push(sibling);
    }
    block
}

fn body_lines<'a>(node: Node, src: &'a [u8], syntax: &CommentSyntax) -> Vec<&'a str> {
    node_text(node, src)
        .unwrap_or_default()
        .lines()
        .map(|line| strip_marker(line, syntax))
        .filter(|line| !line.is_empty())
        .collect()
}

fn strip_marker<'a>(line: &'a str, syntax: &CommentSyntax) -> &'a str {
    let trimmed = line.trim();
    let unclosed = syntax
        .closers
        .iter()
        .find_map(|closer| trimmed.strip_suffix(closer))
        .unwrap_or(trimmed)
        .trim_end();
    syntax
        .markers
        .iter()
        .find_map(|marker| unclosed.strip_prefix(marker))
        .unwrap_or(unclosed)
        .trim()
}

fn looks_like_code(line: &str, syntax: &CommentSyntax) -> bool {
    line.ends_with(syntax.code_endings)
        || syntax
            .code_fragments
            .iter()
            .any(|fragment| line.contains(fragment))
}

fn has_rationale(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.split_whitespace().count() >= RATIONALE_MIN_WORDS
        || RATIONALE_MARKERS
            .iter()
            .any(|marker| has_word(&lower, marker))
        || lower
            .match_indices('#')
            .any(|(at, _)| lower[at + 1..].starts_with(|c: char| c.is_ascii_digit()))
}

fn is_directive(line: &str, syntax: &CommentSyntax) -> bool {
    syntax
        .directives
        .iter()
        .any(|d| line.strip_prefix(d).is_some_and(|rest| ends_word(rest, d)))
}

/// `word` occurs in `text` on word boundaries.
fn has_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(at, _)| {
        let starts = text[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        starts && ends_word(&text[at + word.len()..], word)
    })
}

/// `rest`, the text after `word`, does not continue it (`word` ending in punctuation always ends).
fn ends_word(rest: &str, word: &str) -> bool {
    !word.ends_with(char::is_alphanumeric)
        || rest.chars().next().is_none_or(|c| !c.is_alphanumeric())
}

#[cfg(test)]
mod tests {
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
}

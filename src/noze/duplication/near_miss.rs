//! Opt-in consistent-rename (near-miss Type-2) detection.
//!
//! The default suffix-array matcher is strict: it keeps every name/type/literal
//! verbatim, so renaming an API breaks a clone. This pass relaxes that *only*
//! under a consistent 1:1 renaming — it groups whole functions by an
//! alpha-canonical key that renames identifiers and literals (by first-occurrence
//! ordinal) while keeping operators and control-flow structure verbatim. Two
//! functions share a key iff a consistent rename maps one onto the other.

use crate::report::{ActionLevel, CloneClass, CloneOccurrence};
use crate::spine::parser::tokens::StructuralToken;
use crate::spine::parser::ParsedFile;
use std::collections::HashMap;

/// Tag bit marking a kept (non-renameable) structural/operator code, so it can
/// never collide with a small rename ordinal.
const STRUCT_TAG: u64 = 1 << 63;

struct Member {
    raw: Vec<u64>,
    occ: CloneOccurrence,
    token_len: usize,
}

/// Detect consistent-rename clone classes across `files` (function granularity).
pub fn detect(files: &[&ParsedFile], threshold: usize) -> Vec<CloneClass> {
    let mut groups: HashMap<Vec<u64>, Vec<Member>> = HashMap::new();

    for (idx, file) in files.iter().enumerate() {
        for func in top_level_functions(file) {
            let tokens = function_tokens(file, func);
            if tokens.len() < threshold {
                continue;
            }
            let key = canonical_key(&tokens);
            let raw: Vec<u64> = tokens.iter().map(|(_, lex)| *lex).collect();
            groups.entry(key).or_default().push(Member {
                token_len: tokens.len(),
                raw,
                occ: CloneOccurrence {
                    file: files[idx].path.clone(),
                    start_row: func.start_line,
                    end_row: func.end_line,
                },
            });
        }
    }

    let mut out = Vec::new();
    for members in groups.into_values() {
        if members.len() < 2 {
            continue;
        }
        // Skip groups that are exact clones (already found by the suffix array):
        // a genuine near-miss must have at least two differing raw sequences.
        if members.iter().all(|m| m.raw == members[0].raw) {
            continue;
        }
        let token_length = members.iter().map(|m| m.token_len).min().unwrap_or(0);
        let occurrences: Vec<CloneOccurrence> = members.into_iter().map(|m| m.occ).collect();
        out.push(CloneClass {
            action: ActionLevel::Advisory,
            token_length,
            occurrences,
            hint: Some("consistent-rename clone".to_string()),
        });
    }
    out
}

/// Functions not nested inside another function (methods and module functions).
fn top_level_functions(file: &ParsedFile) -> Vec<&crate::spine::parser::FunctionUnit> {
    let mut functions: Vec<&crate::spine::parser::FunctionUnit> =
        file.walked.units.functions.iter().collect();

    functions.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| b.end_line.cmp(&a.end_line))
    });

    let mut result = Vec::new();
    let mut max_end_line = 0;

    for func in functions {
        if func.end_line > max_end_line {
            result.push(func);
            max_end_line = func.end_line;
        }
    }

    result
}

/// Tokens (kind, lexeme) whose start row falls within the function's lines.
fn function_tokens(
    file: &ParsedFile,
    func: &crate::spine::parser::FunctionUnit,
) -> Vec<(StructuralToken, u64)> {
    file.walked
        .syntax
        .tokens
        .iter()
        .zip(&file.walked.syntax.spans)
        .zip(&file.walked.syntax.lexemes)
        .filter(|((_, span), _)| {
            span.start_row >= func.start_line && span.start_row <= func.end_line
        })
        .map(|((tok, _), lex)| (*tok, *lex))
        .collect()
}

/// Alpha-canonical key: renameable tokens (identifiers/literals) become their
/// first-occurrence ordinal; everything else (operators, calls, control flow)
/// keeps its verbatim code, tagged so the two namespaces never collide.
fn canonical_key(tokens: &[(StructuralToken, u64)]) -> Vec<u64> {
    let mut ordinal: HashMap<u64, u64> = HashMap::new();
    tokens
        .iter()
        .map(|(tok, lex)| {
            if is_renameable(*tok) {
                let next = ordinal.len() as u64;
                *ordinal.entry(*lex).or_insert(next)
            } else {
                lex | STRUCT_TAG
            }
        })
        .collect()
}

fn is_renameable(tok: StructuralToken) -> bool {
    matches!(
        tok,
        StructuralToken::GenericIdentifier | StructuralToken::GenericLiteral
    )
}

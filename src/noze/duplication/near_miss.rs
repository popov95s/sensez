use crate::config::model::Duplication;
use crate::report::{ActionLevel, CloneClass, CloneDetector, CloneOccurrence};
use crate::spine::parser::tokens::StructuralToken;
use crate::spine::parser::ParsedFile;
use std::collections::{BTreeMap, HashMap};

const STRUCT_TAG: u64 = 1 << 63;

struct Member {
    raw: Vec<u64>,
    occ: CloneOccurrence,
    token_len: usize,
}

/// Shortest function the rename pass compares: twice the threshold unless `near_miss` asks for every size.
pub fn min_tokens(config: &Duplication) -> usize {
    match config.near_miss {
        true => config.threshold,
        false => config.threshold * 2,
    }
}

/// Detect consistent-rename clone classes across `files` (function granularity).
pub fn detect(files: &[&ParsedFile], threshold: usize) -> Vec<CloneClass> {
    let mut groups: BTreeMap<Vec<u64>, Vec<Member>> = BTreeMap::new();

    for (idx, file) in files.iter().enumerate() {
        for func in super::top_level_functions(file) {
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
            detector: CloneDetector::NearMiss,
            absorbed: Vec::new(),
        });
    }
    out
}

pub(super) fn function_tokens(
    file: &ParsedFile,
    func: &crate::spine::parser::FunctionUnit,
) -> Vec<(StructuralToken, u64)> {
    let range = super::span_index_range(file, func.start_line, func.end_line);
    file.walked.syntax.tokens[range.clone()]
        .iter()
        .zip(&file.walked.syntax.spans[range.clone()])
        .zip(&file.walked.syntax.lexemes[range])
        .map(|((tok, _), lex)| (*tok, *lex))
        .collect()
}

pub(super) fn canonical_key(tokens: &[(StructuralToken, u64)]) -> Vec<u64> {
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

#[cfg(test)]
#[path = "near_miss_tests.rs"]
mod tests;

//! Comment-backed semantic duplication.
//!
//! This pass is intentionally conservative: structure creates candidates, but
//! comments/docstrings decide whether two candidates share enough documented
//! intent to report. The embedding layer is best at "same thing in words", not
//! at proving code equivalence, so exact/near-miss clone detection remains the
//! high-confidence core.

use crate::config::model::SemanticDuplication;
use crate::eyez;
use crate::report::{ActionLevel, CloneClass, CloneOccurrence};
use crate::spine::parser::tokens::StructuralToken;
use crate::spine::parser::{FunctionUnit, ParsedFile};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::path::PathBuf;

const MIN_TOKENS: usize = 20;

struct Unit {
    file: PathBuf,
    start: usize,
    end: usize,
    tokens: usize,
    shape: BTreeMap<StructuralToken, usize>,
    comment: String,
}

struct Candidate {
    left: usize,
    right: usize,
    shape_score: f32,
}

#[derive(PartialEq, Eq, Hash)]
struct PairKey {
    left_file: PathBuf,
    left_row: usize,
    right_file: PathBuf,
    right_row: usize,
}

pub fn detect(files: &[&ParsedFile], config: &SemanticDuplication) -> Vec<CloneClass> {
    if !config.enabled {
        return Vec::new();
    }
    let units = collect_units(files);
    if units.len() < 2 {
        return Vec::new();
    }
    let candidates = candidate_pairs(&units, config.min_shape_score);
    if candidates.is_empty() {
        return Vec::new();
    }
    let texts: Vec<String> = units.iter().map(|u| u.comment.clone()).collect();
    let Ok(vectors) = eyez::embed_texts(&texts) else {
        return Vec::new();
    };
    findings(units, candidates, &vectors, config.comment_boost_score)
}

fn collect_units(files: &[&ParsedFile]) -> Vec<Unit> {
    let mut out = Vec::new();
    for file in files {
        let comments = comment_bundles(file);
        for func in top_level_functions(file) {
            if let Some(comment) = comment_for(&comments, func) {
                let (tokens, shape) = function_shape(file, func);
                if tokens >= MIN_TOKENS {
                    out.push(Unit {
                        file: file.path.clone(),
                        start: func.start_line,
                        end: func.end_line,
                        tokens,
                        shape,
                        comment,
                    });
                }
            }
        }
    }
    out
}

fn comment_bundles(file: &ParsedFile) -> FxHashMap<String, String> {
    let mut module_context: Vec<&str> = Vec::new();
    let mut by_symbol: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for doc in &file.walked.docs {
        if !doc.symbol_path.contains("::") && doc.line <= 40 {
            module_context.push(doc.text.as_str());
        } else if doc.symbol_path.contains("::") {
            by_symbol
                .entry(doc.symbol_path.clone())
                .or_default()
                .push(doc.text.as_str());
        }
    }
    by_symbol
        .into_iter()
        .map(|(symbol, docs)| {
            let mut parts = module_context.clone();
            parts.extend(docs);
            (symbol, parts.join("\n\n"))
        })
        .collect()
}

fn comment_for(comments: &FxHashMap<String, String>, func: &FunctionUnit) -> Option<String> {
    comments
        .iter()
        .filter(|(symbol, _)| last_segment(symbol) == func.name)
        .map(|(_, text)| text.trim())
        .find(|text| text.split_whitespace().count() >= 5)
        .map(ToOwned::to_owned)
}

fn last_segment(symbol: &str) -> &str {
    symbol
        .rsplit([':', '.'])
        .find(|part| !part.is_empty())
        .unwrap_or(symbol)
}

fn top_level_functions(file: &ParsedFile) -> Vec<&FunctionUnit> {
    file.walked
        .units
        .functions
        .iter()
        .filter(|f| !f.is_nested)
        .collect()
}

fn function_shape(
    file: &ParsedFile,
    func: &FunctionUnit,
) -> (usize, BTreeMap<StructuralToken, usize>) {
    let mut shape = BTreeMap::new();
    let mut count = 0;
    for (tok, span) in file
        .walked
        .syntax
        .tokens
        .iter()
        .zip(&file.walked.syntax.spans)
    {
        if span.start_row >= func.start_line && span.start_row <= func.end_line {
            *shape.entry(*tok).or_insert(0) += 1;
            count += 1;
        }
    }
    (count, shape)
}

fn candidate_pairs(units: &[Unit], min_shape_score: u8) -> Vec<Candidate> {
    let threshold = score_threshold(min_shape_score);
    let mut out = Vec::new();
    for i in 0..units.len() {
        for j in i + 1..units.len() {
            if units[i].file == units[j].file || !similar_size(units[i].tokens, units[j].tokens) {
                continue;
            }
            let shape_score = cosine(&units[i].shape, &units[j].shape);
            if shape_score >= threshold {
                out.push(Candidate {
                    left: i,
                    right: j,
                    shape_score,
                });
            }
        }
    }
    out
}

fn findings(
    units: Vec<Unit>,
    candidates: Vec<Candidate>,
    vectors: &[Vec<f32>],
    min_comment_score: u8,
) -> Vec<CloneClass> {
    let threshold = score_threshold(min_comment_score);
    let mut seen = FxHashSet::default();
    let mut out = Vec::new();
    for candidate in candidates {
        let comment_score = dot(&vectors[candidate.left], &vectors[candidate.right]);
        if comment_score < threshold {
            continue;
        }
        let left = &units[candidate.left];
        let right = &units[candidate.right];
        if !seen.insert(pair_key(left, right)) {
            continue;
        }
        out.push(CloneClass {
            action: ActionLevel::Advisory,
            token_length: left.tokens.min(right.tokens),
            occurrences: vec![occurrence(left), occurrence(right)],
            hint: Some(format!(
                "comment-backed semantic clone: shape {:.2}, comments {:.2}",
                candidate.shape_score, comment_score
            )),
        });
    }
    out
}

fn score_threshold(score: u8) -> f32 {
    (score.min(100) as f32) / 100.0
}

fn similar_size(left: usize, right: usize) -> bool {
    left.min(right) as f32 / left.max(right) as f32 >= 0.55
}

fn cosine(
    left: &BTreeMap<StructuralToken, usize>,
    right: &BTreeMap<StructuralToken, usize>,
) -> f32 {
    let dot: usize = left
        .iter()
        .map(|(tok, count)| count * right.get(tok).copied().unwrap_or(0))
        .sum();
    let left_norm = norm(left);
    let right_norm = norm(right);
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot as f32 / (left_norm * right_norm)
    }
}

fn norm(shape: &BTreeMap<StructuralToken, usize>) -> f32 {
    shape
        .values()
        .map(|count| (count * count) as f32)
        .sum::<f32>()
        .sqrt()
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn occurrence(unit: &Unit) -> CloneOccurrence {
    CloneOccurrence {
        file: unit.file.clone(),
        start_row: unit.start,
        end_row: unit.end,
    }
}

fn pair_key(left: &Unit, right: &Unit) -> PairKey {
    let a = (left.file.clone(), left.start);
    let b = (right.file.clone(), right.start);
    if a <= b {
        PairKey {
            left_file: a.0,
            left_row: a.1,
            right_file: b.0,
            right_row: b.1,
        }
    } else {
        PairKey {
            left_file: b.0,
            left_row: b.1,
            right_file: a.0,
            right_row: a.1,
        }
    }
}

//! Comment-backed semantic duplication.
//!
//! This pass is intentionally conservative: structure creates candidates, but
//! comments/docstrings decide whether two candidates share enough documented
//! intent to report. The embedding layer is best at "same thing in words", not
//! at proving code equivalence, so exact/near-miss clone detection remains the
//! high-confidence core.

mod keying;

use crate::config::model::SemanticDuplication;
use crate::eyez;
use crate::eyez::semantic_cache::BundleInput;
use crate::report::{ActionLevel, CloneClass, CloneOccurrence};
use crate::spine::parser::tokens::StructuralToken;
use crate::spine::parser::{FunctionUnit, ParsedFile};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use keying::{bundle_key, pair_key};
const MIN_SIZE_RATIO: f32 = 0.55;
const MIN_TOKENS: usize = 20;

struct Unit {
    file: PathBuf,
    start: usize,
    end: usize,
    tokens: usize,
    key: u64,
    shape: BTreeMap<StructuralToken, usize>,
    comment: String,
}

struct Candidate {
    left: usize,
    right: usize,
    shape_score: f32,
}

pub fn detect(
    files: &[&ParsedFile],
    config: &SemanticDuplication,
    root: Option<&Path>,
) -> Vec<CloneClass> {
    if !config.enabled {
        return Vec::new();
    }
    let units = collect_units(files, config.comment_required);
    if units.len() < 2 {
        return Vec::new();
    }
    let candidates = candidate_pairs(&units, config.min_shape_score);
    if candidates.is_empty() {
        return Vec::new();
    }
    let vectors = match vectors_for(root, &units) {
        Ok(vectors) if vectors.len() == units.len() => vectors,
        Ok(vectors) => {
            eprintln!(
                "[sensez] semantic duplication: embedding returned {}/{} vectors; skipping pass",
                vectors.len(),
                units.len()
            );
            return Vec::new();
        }
        Err(err) => {
            eprintln!("[sensez] semantic duplication unavailable: {err:#}");
            return Vec::new();
        }
    };
    findings(units, candidates, &vectors, config.comment_boost_score)
}

fn collect_units(files: &[&ParsedFile], comment_required: bool) -> Vec<Unit> {
    let mut out = Vec::new();
    for file in files {
        let file_hash = file.fingerprint.content;
        let comments = comment_bundles(file);
        for func in super::top_level_functions(file) {
            if let Some((symbol_path, comment)) = comment_for(&comments, func, comment_required) {
                let (tokens, shape) = function_shape(file, func);
                if tokens >= MIN_TOKENS {
                    let key = bundle_key(
                        file_hash,
                        &file.path,
                        &symbol_path,
                        func,
                        tokens,
                        &shape,
                        &comment,
                    );
                    out.push(Unit {
                        file: file.path.clone(),
                        start: func.start_line,
                        end: func.end_line,
                        tokens,
                        key,
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

fn comment_for(
    comments: &FxHashMap<String, String>,
    func: &FunctionUnit,
    comment_required: bool,
) -> Option<(String, String)> {
    let commented = comments
        .iter()
        .filter(|(symbol, _)| last_segment(symbol) == func.name)
        .map(|(symbol, text)| (symbol.as_str(), text.trim()))
        .find(|(_, text)| text.split_whitespace().count() >= 5)
        .map(|(symbol, text)| (symbol.to_owned(), text.to_owned()));
    if commented.is_some() || comment_required {
        return commented;
    }
    Some((func.name.clone(), format!("function {}", func.name)))
}

fn last_segment(symbol: &str) -> &str {
    symbol
        .rsplit([':', '.'])
        .find(|part| !part.is_empty())
        .unwrap_or(symbol)
}

fn function_shape(
    file: &ParsedFile,
    func: &FunctionUnit,
) -> (usize, BTreeMap<StructuralToken, usize>) {
    let range = super::span_index_range(file, func.start_line, func.end_line);
    let mut shape = BTreeMap::new();
    for tok in &file.walked.syntax.tokens[range] {
        *shape.entry(*tok).or_insert(0) += 1;
    }
    (shape.values().sum(), shape)
}

fn candidate_pairs(units: &[Unit], min_shape_score: u8) -> Vec<Candidate> {
    let threshold = score_threshold(min_shape_score);
    let mut order: Vec<usize> = (0..units.len()).collect();
    order.sort_by_key(|&index| units[index].tokens);

    let mut out = Vec::new();
    for (pos, &left) in order.iter().enumerate() {
        let max_tokens = (units[left].tokens as f32 / MIN_SIZE_RATIO).ceil() as usize;
        for &right in &order[pos + 1..] {
            if units[right].tokens > max_tokens {
                break; // sorted: every later unit is at least as big
            }
            if units[left].file == units[right].file {
                continue;
            }
            let shape_score = cosine(&units[left].shape, &units[right].shape);
            if shape_score >= threshold {
                // Candidate indices must be in unit order (they index both
                // `units` and the aligned vector list).
                let (a, b) = if left < right {
                    (left, right)
                } else {
                    (right, left)
                };
                out.push(Candidate {
                    left: a,
                    right: b,
                    shape_score,
                });
            }
        }
    }
    out
}

fn vectors_for(root: Option<&Path>, units: &[Unit]) -> anyhow::Result<Vec<Vec<f32>>> {
    let mut inputs = Vec::with_capacity(units.len());
    let mut texts = Vec::with_capacity(units.len());
    for unit in units {
        let text = unit.comment.clone();
        inputs.push(BundleInput {
            key: unit.key,
            text: text.clone(),
        });
        texts.push(text);
    }
    match root {
        Some(root) => eyez::semantic_vectors(root, &inputs),
        None => eyez::embed_texts(&texts),
    }
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

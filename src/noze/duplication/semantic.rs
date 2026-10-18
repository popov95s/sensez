//! Semantic duplication: edited copies and same-intent rewrites.
//!
//! Structure is 4-gram Jaccard over the alpha-canonical token stream; doc
//! comments are compared by embedding. The embedding layer is best at "same
//! thing in words", not at proving code equivalence, so exact/near-miss clone
//! detection remains the high-confidence core.

mod candidates;
mod keying;
mod shingles;
mod verdict;

use crate::config::model::SemanticDuplication;
use crate::eyez;
use crate::eyez::semantic_cache::BundleInput;
use crate::eyez::DocKind;
use crate::report::{CloneClass, CloneDetector, CloneOccurrence};
use crate::spine::parser::{FunctionUnit, ParsedFile};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use candidates::similar_pairs;
use keying::{bundle_key, pair_key};
use shingles::{jaccard, shingles, words};
const MIN_SIZE_RATIO: f32 = 0.55;
/// Doc-word Jaccard a pair needs before its doc comments are embedded and compared.
const MIN_DOC_WORD_OVERLAP: f32 = 0.5;

struct Unit {
    file: PathBuf,
    start: usize,
    end: usize,
    tokens: usize,
    shingles: FxHashSet<u64>,
    doc: Option<UnitDoc>,
}

struct UnitDoc {
    key: u64,
    text: String,
    words: FxHashSet<u64>,
}

pub fn detect(
    files: &[&ParsedFile],
    config: &SemanticDuplication,
    threshold: usize,
    root: Option<&Path>,
) -> anyhow::Result<Vec<CloneClass>> {
    if !config.enabled {
        return Ok(Vec::new());
    }
    let units = collect_units(files, config, threshold);
    let candidates = candidate_pairs(&units, config);
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let vectors = vectors_for(root, &units, &candidates)?;
    Ok(findings(&units, candidates, &vectors, config))
}

fn collect_units(
    files: &[&ParsedFile],
    config: &SemanticDuplication,
    threshold: usize,
) -> Vec<Unit> {
    let mut out = Vec::new();
    for file in files {
        let file_hash = file.fingerprint.content;
        let comments = comment_bundles(file);
        for func in super::top_level_functions(file) {
            let comment = comment_for(&comments, func);
            if comment.is_none() && config.comment_required {
                continue;
            }
            let tokens = super::near_miss::function_tokens(file, func);
            if tokens.len() < threshold {
                continue;
            }
            let shingles = shingles(&tokens);
            let doc = comment.map(|(symbol_path, text)| UnitDoc {
                words: words(&text),
                key: bundle_key(
                    file_hash,
                    &file.path,
                    &symbol_path,
                    func,
                    tokens.len(),
                    &shingles,
                    &text,
                ),
                text,
            });
            out.push(Unit {
                file: file.path.clone(),
                start: func.start_line,
                end: func.end_line,
                tokens: tokens.len(),
                shingles,
                doc,
            });
        }
    }
    out
}

fn comment_bundles(file: &ParsedFile) -> FxHashMap<String, (usize, DocKind, String)> {
    let mut by_symbol: BTreeMap<String, Vec<&crate::eyez::RawDoc>> = BTreeMap::new();
    for doc in &file.walked.docs {
        if doc.symbol_path.contains("::") {
            by_symbol
                .entry(doc.symbol_path.clone())
                .or_default()
                .push(doc);
        }
    }
    by_symbol
        .into_iter()
        .map(|(symbol, docs)| {
            (
                symbol,
                (
                    docs.last().map_or(0, |doc| doc.line),
                    docs.last().map_or(DocKind::Comment, |doc| doc.kind),
                    docs.iter()
                        .map(|doc| doc.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                ),
            )
        })
        .collect()
}

fn comment_for(
    comments: &FxHashMap<String, (usize, DocKind, String)>,
    func: &FunctionUnit,
) -> Option<(String, String)> {
    comments
        .iter()
        .filter(|(symbol, (line, kind, text))| {
            last_segment(symbol) == func.name
                && (match kind {
                    DocKind::Docstring => *line >= func.start_line && *line <= func.end_line,
                    DocKind::Comment => *line < func.start_line,
                })
                && text.split_whitespace().count() >= 5
        })
        .min_by_key(|(_, (line, _, _))| line.abs_diff(func.start_line))
        .map(|(symbol, (_, _, text))| (symbol.to_owned(), text.trim().to_owned()))
}

fn last_segment(symbol: &str) -> &str {
    symbol
        .rsplit([':', '.'])
        .find(|part| !part.is_empty())
        .unwrap_or(symbol)
}

/// Cross-file, size-compatible pairs sharing enough structure or doc words to possibly report.
fn candidate_pairs(units: &[Unit], config: &SemanticDuplication) -> Vec<(usize, usize)> {
    let no_words = FxHashSet::default();
    let shingles: Vec<&FxHashSet<u64>> = units.iter().map(|unit| &unit.shingles).collect();
    let words: Vec<&FxHashSet<u64>> = units
        .iter()
        .map(|unit| unit.doc.as_ref().map_or(&no_words, |doc| &doc.words))
        .collect();
    let mut pairs = similar_pairs(&shingles, verdict::min_structure(config));
    pairs.extend(similar_pairs(&words, MIN_DOC_WORD_OVERLAP));
    pairs
        .into_iter()
        .filter(|&(left, right)| size_compatible(&units[left], &units[right]))
        .collect()
}

fn size_compatible(left: &Unit, right: &Unit) -> bool {
    let (small, large) = (left.tokens.min(right.tokens), left.tokens.max(right.tokens));
    left.file != right.file && small as f32 >= large as f32 * MIN_SIZE_RATIO
}

/// Doc vectors aligned with `units`, only for documented units paired with another documented unit.
fn vectors_for(
    root: Option<&Path>,
    units: &[Unit],
    candidates: &[(usize, usize)],
) -> anyhow::Result<Vec<Option<Vec<f32>>>> {
    let needed: BTreeSet<usize> = candidates
        .iter()
        .filter(|&&(left, right)| units[left].doc.is_some() && units[right].doc.is_some())
        .flat_map(|&(left, right)| [left, right])
        .collect();
    let inputs: Vec<BundleInput> = needed
        .iter()
        .filter_map(|&index| units[index].doc.as_ref())
        .map(|doc| BundleInput {
            key: doc.key,
            text: doc.text.clone(),
        })
        .collect();
    let mut aligned: Vec<Option<Vec<f32>>> = units.iter().map(|_| None).collect();
    if inputs.is_empty() {
        return Ok(aligned);
    }
    let vectors = match root {
        Some(root) => eyez::semantic_vectors(root, &inputs)?,
        None => {
            let texts: Vec<String> = inputs.iter().map(|input| input.text.clone()).collect();
            eyez::embed_texts(&texts)?
        }
    };
    if vectors.len() != inputs.len() {
        anyhow::bail!(
            "embedding returned {}/{} vectors",
            vectors.len(),
            inputs.len()
        );
    }
    for (index, vector) in needed.into_iter().zip(vectors) {
        aligned[index] = Some(vector);
    }
    Ok(aligned)
}

fn findings(
    units: &[Unit],
    candidates: Vec<(usize, usize)>,
    vectors: &[Option<Vec<f32>>],
    config: &SemanticDuplication,
) -> Vec<CloneClass> {
    let mut seen = FxHashSet::default();
    let mut out = Vec::new();
    for (left_index, right_index) in candidates {
        let left = &units[left_index];
        let right = &units[right_index];
        let docs = match (&left.doc, &right.doc) {
            (Some(_), Some(_)) => match (&vectors[left_index], &vectors[right_index]) {
                (Some(left), Some(right)) => Some(dot(left, right)),
                _ => continue,
            },
            (None, None) => None,
            _ => continue,
        };
        let structure = jaccard(&left.shingles, &right.shingles);
        let Some(verdict) = verdict::verdict(structure, docs, config) else {
            continue;
        };
        if !seen.insert(pair_key(left, right)) {
            continue;
        }
        out.push(CloneClass {
            action: verdict::action(verdict),
            token_length: left.tokens.min(right.tokens),
            occurrences: vec![occurrence(left), occurrence(right)],
            hint: Some(verdict::hint(verdict, structure, docs)),
            detector: CloneDetector::Semantic { verdict },
            absorbed: Vec::new(),
        });
    }
    out
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

#[cfg(test)]
mod tests;

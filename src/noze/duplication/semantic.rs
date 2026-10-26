//! Semantic duplication: edited copies and same-intent rewrites.
//!
//! Structure is 4-gram Jaccard over the alpha-canonical token stream. Whole
//! function source, the words of the function's signature and its doc comment
//! are each compared by embedding with a code model.

mod candidates;
mod intent;
mod keying;
mod shingles;
mod vectors;
mod verdict;

use crate::config::model::SemanticDuplication;
use crate::eyez::DocKind;
use crate::report::{CloneClass, CloneDetector, CloneOccurrence, ScanIssue, ScanStage};
use crate::spine::parser::{FunctionUnit, ParsedFile};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use candidates::{common_items, similar_pairs, without_common};
use keying::text_key;
use shingles::{jaccard, shingles, words};
use vectors::{vectors_for, Slot, Vectors};
use verdict::Scores;
const MIN_SIZE_RATIO: f32 = 0.55;
/// Doc-word Jaccard a pair needs before its doc comments are embedded and compared.
const MIN_DOC_WORD_OVERLAP: f32 = 0.5;
/// Items (shingles, doc words) in more than this share of units, and in more than a handful, cannot make a pair a candidate.
const MAX_COMMON_SHARE: f32 = 0.01;
const MIN_COMMON_USES: usize = 8;
/// Signature-word Jaccard a pair needs before it is compared at all.
const MIN_NAME_WORD_OVERLAP: f32 = 0.3;
/// A signature word used by more than this share of functions (and by more than a handful) says nothing about intent.
const MAX_WORD_SHARE: f32 = 0.02;
const MIN_COMMON_WORD_USES: usize = 4;

struct Unit {
    file: PathBuf,
    start: usize,
    end: usize,
    tokens: usize,
    shingles: FxHashSet<u64>,
    doc: Option<UnitText>,
    signature_words: Vec<String>,
    signature: Option<UnitText>,
    source: Rc<str>,
}

impl Unit {
    /// Whole-function source; built only for units that get embedded.
    fn code(&self) -> String {
        self.source
            .lines()
            .skip(self.start.saturating_sub(1))
            .take((self.end + 1).saturating_sub(self.start.max(1)))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

struct UnitText {
    key: u64,
    text: String,
    words: FxHashSet<u64>,
}

pub fn detect(
    files: &[&ParsedFile],
    config: &SemanticDuplication,
    threshold: usize,
    root: Option<&Path>,
) -> anyhow::Result<(Vec<CloneClass>, Vec<ScanIssue>)> {
    if !config.enabled {
        return Ok((Vec::new(), Vec::new()));
    }
    let (mut units, issues) = collect_units(files, config, threshold);
    keep_distinctive_signatures(&mut units);
    let candidates = candidate_pairs(&units, config);
    if candidates.is_empty() {
        return Ok((Vec::new(), issues));
    }
    let vectors = vectors_for(root, &units, &candidates)?;
    Ok((findings(&units, candidates, &vectors, config), issues))
}

fn collect_units(
    files: &[&ParsedFile],
    config: &SemanticDuplication,
    threshold: usize,
) -> (Vec<Unit>, Vec<ScanIssue>) {
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for file in files {
        let source: Rc<str> = match std::fs::read(&file.path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into(),
            Err(err) => {
                issues.push(unreadable(&file.path, &err));
                continue;
            }
        };
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
            out.push(Unit {
                file: file.path.clone(),
                start: func.start_line,
                end: func.end_line,
                tokens: tokens.len(),
                shingles: shingles(&tokens),
                doc: comment.map(unit_text),
                signature_words: intent::signature_words(func),
                signature: None,
                source: Rc::clone(&source),
            });
        }
    }
    (out, issues)
}

fn unreadable(path: &Path, err: &std::io::Error) -> ScanIssue {
    ScanIssue {
        stage: ScanStage::Analyze,
        file: Some(path.to_path_buf()),
        message: format!("duplication.semantic skipped {}: {err}", path.display()),
    }
}

/// Signatures built only from words few functions in the codebase use.
fn keep_distinctive_signatures(units: &mut [Unit]) {
    let common = common_items(
        units.iter().map(|unit| &unit.signature_words),
        MAX_WORD_SHARE,
        MIN_COMMON_WORD_USES,
    );
    for unit in units.iter_mut() {
        let words: Vec<&str> = unit
            .signature_words
            .iter()
            .filter(|word| !common.contains(*word))
            .map(String::as_str)
            .collect();
        unit.signature = (words.len() >= 2).then(|| unit_text(words.join(" ")));
    }
}

fn unit_text(text: String) -> UnitText {
    UnitText {
        key: text_key(&text),
        words: words(&text),
        text,
    }
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
) -> Option<String> {
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
        .map(|(_, (_, _, text))| text.trim().to_owned())
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
    let doc_words: Vec<&FxHashSet<u64>> = units
        .iter()
        .map(|unit| unit.doc.as_ref().map_or(&no_words, |doc| &doc.words))
        .collect();
    let name_words: Vec<&FxHashSet<u64>> = units
        .iter()
        .map(|unit| {
            unit.signature
                .as_ref()
                .map_or(&no_words, |name| &name.words)
        })
        .collect();
    let mut pairs = informative_pairs(&shingles, verdict::min_structure(config));
    pairs.extend(informative_pairs(&doc_words, MIN_DOC_WORD_OVERLAP));
    pairs.extend(similar_pairs(&name_words, MIN_NAME_WORD_OVERLAP));
    pairs
        .into_iter()
        .filter(|&(left, right)| size_compatible(&units[left], &units[right]))
        .collect()
}

/// Pairs that share enough of their uncommon items; items most units share would pair everything with everything.
fn informative_pairs(sets: &[&FxHashSet<u64>], min_jaccard: f32) -> BTreeSet<(usize, usize)> {
    let pruned = without_common(sets, MAX_COMMON_SHARE, MIN_COMMON_USES);
    let refs: Vec<&FxHashSet<u64>> = pruned.iter().collect();
    similar_pairs(&refs, min_jaccard)
}

fn size_compatible(left: &Unit, right: &Unit) -> bool {
    let (small, large) = (left.tokens.min(right.tokens), left.tokens.max(right.tokens));
    left.file != right.file && small as f32 >= large as f32 * MIN_SIZE_RATIO
}

fn findings(
    units: &[Unit],
    candidates: Vec<(usize, usize)>,
    vectors: &Vectors,
    config: &SemanticDuplication,
) -> Vec<CloneClass> {
    let mut out = Vec::new();
    for (left_index, right_index) in candidates {
        let left = &units[left_index];
        let right = &units[right_index];
        let similarity = |slot| cosine(vectors, slot, left_index, right_index);
        let scores = Scores {
            structure: jaccard(&left.shingles, &right.shingles),
            code: similarity(Slot::Code),
            names: similarity(Slot::Name),
            docs: similarity(Slot::Doc),
        };
        let Some(verdict) = verdict::verdict(&scores, config) else {
            continue;
        };
        out.push(CloneClass {
            action: verdict::action(verdict),
            token_length: left.tokens.min(right.tokens),
            occurrences: vec![occurrence(left), occurrence(right)],
            hint: Some(verdict::hint(verdict, &scores)),
            detector: CloneDetector::Semantic { verdict },
            absorbed: Vec::new(),
        });
    }
    out
}

/// Dot product of two unit-norm embeddings; `None` when either is missing or not finite.
fn cosine(vectors: &Vectors, slot: Slot, left: usize, right: usize) -> Option<f32> {
    let (left, right) = (vectors.get(&(left, slot))?, vectors.get(&(right, slot))?);
    let dot: f32 = left.iter().zip(right).map(|(a, b)| a * b).sum();
    dot.is_finite().then_some(dot)
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

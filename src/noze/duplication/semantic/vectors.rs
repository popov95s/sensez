use super::keying::text_key;
use super::Unit;
use crate::eyez;
use crate::eyez::semantic_cache::BundleInput;
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;
use std::path::Path;

/// Embedding of one unit's text in one slot.
pub(super) type Vectors = FxHashMap<(usize, Slot), Vec<f32>>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Slot {
    Doc,
    Name,
    Code,
}

/// Doc, signature and source vectors of every unit in a candidate pair.
pub(super) fn vectors_for(
    root: Option<&Path>,
    units: &[Unit],
    candidates: &[(usize, usize)],
) -> anyhow::Result<Vectors> {
    let needed: BTreeSet<usize> = candidates
        .iter()
        .flat_map(|&(left, right)| [left, right])
        .collect();
    let codes: Vec<String> = needed.iter().map(|&index| units[index].code()).collect();
    let mut slots = Vec::new();
    let mut inputs = Vec::new();
    for (&index, code) in needed.iter().zip(&codes) {
        let unit = &units[index];
        let texts = [
            (
                Slot::Doc,
                unit.doc.as_ref().map(|doc| (doc.key, doc.text.as_str())),
            ),
            (
                Slot::Name,
                unit.signature
                    .as_ref()
                    .map(|name| (name.key, name.text.as_str())),
            ),
            (Slot::Code, Some((text_key(code), code.as_str()))),
        ];
        for (slot, text) in texts {
            if let Some((key, text)) = text {
                slots.push((index, slot));
                inputs.push(BundleInput { key, text });
            }
        }
    }
    let embedded = embed(root, &inputs)?;
    if embedded.len() != inputs.len() {
        anyhow::bail!(
            "embedding returned {}/{} vectors",
            embedded.len(),
            inputs.len()
        );
    }
    Ok(slots.into_iter().zip(embedded).collect())
}

fn embed(root: Option<&Path>, inputs: &[BundleInput]) -> anyhow::Result<Vec<Vec<f32>>> {
    match root {
        Some(root) => eyez::semantic_vectors(root, inputs),
        None => {
            let texts: Vec<String> = inputs.iter().map(|input| input.text.to_owned()).collect();
            eyez::embed_texts(&texts)
        }
    }
}

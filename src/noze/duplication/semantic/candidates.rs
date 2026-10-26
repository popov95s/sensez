use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeSet;
use std::hash::Hash;

/// Slack so float error never raises the required overlap by one item.
const JACCARD_EPSILON: f32 = 1e-4;

/// Every `(a, b)` with `a < b` whose sets may reach `min_jaccard`, via prefix filtering on an inverted index.
pub(super) fn similar_pairs(
    sets: &[&FxHashSet<u64>],
    min_jaccard: f32,
) -> BTreeSet<(usize, usize)> {
    let mut frequency: FxHashMap<u64, usize> = FxHashMap::default();
    for set in sets {
        for &item in *set {
            *frequency.entry(item).or_default() += 1;
        }
    }
    let mut index: FxHashMap<u64, Vec<usize>> = FxHashMap::default();
    let mut pairs = BTreeSet::new();
    for (unit, set) in sets.iter().enumerate() {
        for item in rarest_first(set, &frequency)
            .into_iter()
            .take(prefix_len(set.len(), min_jaccard))
        {
            let posting = index.entry(item).or_default();
            pairs.extend(posting.iter().map(|&earlier| (earlier, unit)));
            posting.push(unit);
        }
    }
    pairs
}

/// Each set without the items more than `max_share` of all sets (and more than `min_uses`) contain.
pub(super) fn without_common(
    sets: &[&FxHashSet<u64>],
    max_share: f32,
    min_uses: usize,
) -> Vec<FxHashSet<u64>> {
    let common = common_items(sets.iter().copied(), max_share, min_uses);
    sets.iter()
        .map(|set| set.difference(&common).copied().collect())
        .collect()
}

/// Items in more than `max_share` of `groups` and in more than `min_uses` of them.
pub(super) fn common_items<'a, T: Hash + Eq + Clone + 'a>(
    groups: impl ExactSizeIterator<Item = impl IntoIterator<Item = &'a T>>,
    max_share: f32,
    min_uses: usize,
) -> FxHashSet<T> {
    let limit = min_uses.max((groups.len() as f32 * max_share).floor() as usize);
    let mut frequency: FxHashMap<&T, usize> = FxHashMap::default();
    for group in groups {
        for item in group {
            *frequency.entry(item).or_default() += 1;
        }
    }
    frequency
        .into_iter()
        .filter(|&(_, uses)| uses > limit)
        .map(|(item, _)| item.clone())
        .collect()
}

/// `set` ordered by corpus frequency, rarest first (ties by value for determinism).
fn rarest_first(set: &FxHashSet<u64>, frequency: &FxHashMap<u64, usize>) -> Vec<u64> {
    let mut items: Vec<u64> = set.iter().copied().collect();
    items.sort_unstable_by_key(|item| (frequency[item], *item));
    items
}

/// Two sets with Jaccard >= t share an item within the first `n - ceil(t * n) + 1` rarest items of each.
fn prefix_len(len: usize, min_jaccard: f32) -> usize {
    let required = (min_jaccard * len as f32 - JACCARD_EPSILON).ceil() as usize;
    (len + 1).saturating_sub(required).min(len)
}

#[cfg(test)]
#[path = "candidates_tests.rs"]
mod tests;

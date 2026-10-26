use super::*;
use crate::noze::duplication::semantic::shingles::jaccard;

fn set(items: impl IntoIterator<Item = u64>) -> FxHashSet<u64> {
    items.into_iter().collect()
}

#[test]
fn finds_every_pair_at_or_above_the_threshold() {
    let sets: Vec<FxHashSet<u64>> = (0..30u64)
        .map(|seed| set((0..20).map(|i| (seed * 7 + i * (seed % 3 + 1)) % 40)))
        .collect();
    let refs: Vec<&FxHashSet<u64>> = sets.iter().collect();
    for threshold in [0.2, 0.35, 0.5, 0.8] {
        let found = similar_pairs(&refs, threshold);
        for a in 0..sets.len() {
            for b in a + 1..sets.len() {
                if jaccard(&sets[a], &sets[b]) >= threshold {
                    assert!(found.contains(&(a, b)), "missed ({a}, {b}) at {threshold}");
                }
            }
        }
    }
}

#[test]
fn disjoint_sets_are_never_paired() {
    let left = set(0..10);
    let right = set(100..110);
    assert!(similar_pairs(&[&left, &right], 0.0).is_empty());
}

#[test]
fn prefix_shrinks_as_the_threshold_rises() {
    assert_eq!(prefix_len(10, 0.0), 10);
    assert_eq!(prefix_len(10, 0.3), 8);
    assert_eq!(prefix_len(10, 1.0), 1);
    assert_eq!(prefix_len(0, 0.5), 0);
}

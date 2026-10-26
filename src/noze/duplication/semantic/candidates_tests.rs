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

#[test]
fn items_most_sets_share_cannot_pair_sets() {
    let common = 999;
    let sets: Vec<FxHashSet<u64>> = (0..20u64).map(|i| set([common, i])).collect();
    let refs: Vec<&FxHashSet<u64>> = sets.iter().collect();
    let pruned = without_common(&refs, 0.5, 2);
    assert!(pruned.iter().all(|set| !set.contains(&common)));
    let pruned_refs: Vec<&FxHashSet<u64>> = pruned.iter().collect();
    assert!(similar_pairs(&pruned_refs, 0.3).is_empty());
}

#[test]
fn common_items_limit_floors_the_share() {
    let groups: Vec<Vec<u64>> = (0..10u64)
        .map(|i| match i {
            0..=1 => vec![1, 2],
            2 => vec![1],
            _ => vec![i + 10],
        })
        .collect();
    let common = common_items(groups.iter(), 0.25, 1);
    assert_eq!(
        common,
        FxHashSet::from_iter([1]),
        "2.5 floors to 2: three uses are common, two are not"
    );
}

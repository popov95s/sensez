//! Most sites that can run on one execution path, given their branch arms.

use std::collections::BTreeMap;

type Arm = (usize, usize);

/// Largest set of sites no two of which sit in different arms of one conditional.
pub(super) fn max_on_one_path<P: AsRef<[Arm]>>(paths: &[P]) -> usize {
    let mut here = 0;
    let mut arms: BTreeMap<usize, BTreeMap<usize, Vec<&[Arm]>>> = BTreeMap::new();
    for path in paths {
        match path.as_ref().split_first() {
            None => here += 1,
            Some((&(conditional, arm), rest)) => arms
                .entry(conditional)
                .or_default()
                .entry(arm)
                .or_default()
                .push(rest),
        }
    }
    let nested: usize = arms
        .values()
        .map(|by_arm| {
            by_arm
                .values()
                .map(|rest| max_on_one_path(rest))
                .max()
                .unwrap_or(0)
        })
        .sum();
    here + nested
}

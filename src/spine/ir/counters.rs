//! Allocation-free updates of name-keyed counters and sets.

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

/// Increment `map[key]`, allocating the key only when absent.
pub(crate) fn bump<S: BuildHasher>(map: &mut HashMap<String, usize, S>, key: &str) {
    if let Some(count) = map.get_mut(key) {
        *count += 1;
    } else {
        map.insert(key.to_string(), 1);
    }
}

/// Record that `attr` was accessed on `base`, allocating only on first sight.
pub(crate) fn record_attr<S: BuildHasher, T: BuildHasher + Default>(
    map: &mut HashMap<String, HashSet<String, T>, S>,
    base: &str,
    attr: &str,
) {
    match map.get_mut(base) {
        Some(attrs) => {
            if !attrs.contains(attr) {
                attrs.insert(attr.to_string());
            }
        }
        None => {
            let mut attrs = HashSet::default();
            attrs.insert(attr.to_string());
            map.insert(base.to_string(), attrs);
        }
    }
}

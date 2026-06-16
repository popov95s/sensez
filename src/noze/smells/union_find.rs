//! Minimal slice-backed union-find (path-halving, no rank) shared by the
//! data-clump and cohesion (LCOM) detectors. Component counts stay small
//! (methods of one class / params of one function), so amortized O(α(n))
//! without union-by-rank is plenty.

/// Root of `x`, compressing the path as it walks.
pub(super) fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// Merge the components containing `a` and `b`.
pub(super) fn union(parent: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        parent[ra] = rb;
    }
}

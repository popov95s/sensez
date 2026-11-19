/// Root of `x`, compressing the path as it walks.
pub(super) fn find(parent: &mut [usize], x: usize) -> usize {
    let mut node = x;
    while parent[node] != node {
        parent[node] = parent[parent[node]];
        node = parent[node];
    }
    node
}

/// Merge the components containing `a` and `b`.
pub(super) fn union(parent: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        parent[ra] = rb;
    }
}

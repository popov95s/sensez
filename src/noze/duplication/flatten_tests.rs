use super::*;

/// `densify` must yield a gap-free `0..K` alphabet with the terminator (raw
/// `0`) pinned to id `0` and a consistent bijection (equal raws → equal ids,
/// distinct raws → distinct ids). These are exactly bio's `suffix_array_int`
/// preconditions; if this regresses, the SA panics or mis-groups clones.
#[test]
fn densify_is_contiguous_bijection_with_zero_sentinel() {
    // Terminator 0 last; high separator values interleaved with real codes.
    let raw: Vec<u64> = vec![50, 17, 50, 1 << 62, 17, 99, 50, 0];
    let dense = densify(&raw);

    // Terminator → smallest id.
    assert_eq!(dense[raw.len() - 1], 0, "sentinel must be the smallest id");

    // Contiguous 0..K: the set of ids is exactly {0, 1, ..., K-1}.
    let mut ids: Vec<i32> = dense.clone();
    ids.sort_unstable();
    ids.dedup();
    let k = ids.len();
    assert_eq!(
        ids,
        (0..k as i32).collect::<Vec<_>>(),
        "alphabet must be gap-free"
    );

    // Bijection: positions are equal in `dense` iff equal in `raw`.
    for i in 0..raw.len() {
        for j in 0..raw.len() {
            assert_eq!(
                raw[i] == raw[j],
                dense[i] == dense[j],
                "relabeling must preserve the equality structure"
            );
        }
    }
}

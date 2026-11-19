use super::*;

#[test]
fn densify_is_contiguous_bijection_with_zero_sentinel() {
    let raw: Vec<u64> = vec![50, 17, 50, 1 << 62, 17, 99, 50, 0];
    let dense = densify(&raw);

    assert_eq!(dense[raw.len() - 1], 0, "sentinel must be the smallest id");

    let mut ids: Vec<i32> = dense.clone();
    ids.sort_unstable();
    ids.dedup();
    let k = ids.len();
    assert_eq!(
        ids,
        (0..k as i32).collect::<Vec<_>>(),
        "alphabet must be gap-free"
    );

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

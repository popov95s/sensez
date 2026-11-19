use super::*;
use crate::spine::ir::FileId;

fn spans() -> Vec<TokenSpan> {
    let f = |file_id: FileId| TokenSpan {
        file_id,
        start_row: 1,
        end_row: 1,
        start_byte: 0,
        end_byte: 0,
    };
    let mut s: Vec<TokenSpan> = (0..6).map(|_| f(FileId::from_index(0))).collect();
    s.push(f(FileId::SEPARATOR));
    s.extend((0..6).map(|_| f(FileId::from_index(1))));
    s
}

fn groups() -> Vec<RawGroup> {
    vec![
        RawGroup {
            len: 2,
            positions: vec![0, 7],
        },
        RawGroup {
            len: 2,
            positions: vec![3, 11],
        },
    ]
}

#[test]
fn stitches_consistent_gap() {
    let clusters = stitch(groups(), &spans(), 3);
    assert_eq!(clusters.len(), 1, "two adjacent clones merge into one");
    assert_eq!(clusters[0].matched, 4, "matched excludes the gap");
    assert_eq!(clusters[0].occ.len(), 2);
    assert!(clusters[0].occ.contains(&(0, 5)));
    assert!(clusters[0].occ.contains(&(7, 13)));
}

#[test]
fn does_not_stitch_when_gap_exceeds_budget() {
    let clusters = stitch(groups(), &spans(), 1);
    assert_eq!(clusters.len(), 2, "inconsistent/over-budget gap → no merge");
}

#[test]
fn max_gap_zero_keeps_exact_clones() {
    let clusters = stitch(groups(), &spans(), 0);
    assert_eq!(clusters.len(), 2);
    assert!(clusters.iter().all(|c| c.matched == 2));
}

#[test]
fn shared_start_picks_the_same_sized_follower() {
    let mut clusters = groups();
    clusters.push(RawGroup {
        len: 1,
        positions: vec![3, 11, 12],
    });
    let stitched = stitch(clusters, &spans(), 3);
    assert!(stitched
        .iter()
        .any(|c| c.matched == 4 && c.occ == vec![(0, 5), (7, 13)]));
}

#[test]
fn overlapping_follower_is_stitched_without_double_counting() {
    let clusters = vec![
        RawGroup {
            len: 3,
            positions: vec![0, 7],
        },
        RawGroup {
            len: 3,
            positions: vec![2, 10],
        },
    ];
    let stitched = stitch(clusters, &spans(), 3);
    assert_eq!(stitched.len(), 1);
    assert_eq!(stitched[0].matched, 5);
    assert_eq!(stitched[0].occ, vec![(0, 5), (7, 13)]);
}

#[test]
fn sparse_stitch_fails_the_density_guard() {
    let f = |file_id: FileId| TokenSpan {
        file_id,
        start_row: 1,
        end_row: 1,
        start_byte: 0,
        end_byte: 0,
    };
    let mut spans: Vec<TokenSpan> = (0..8).map(|_| f(FileId::from_index(0))).collect();
    spans.push(f(FileId::SEPARATOR));
    spans.extend((0..8).map(|_| f(FileId::from_index(1))));
    let clusters = vec![
        RawGroup {
            len: 2,
            positions: vec![0, 9],
        },
        RawGroup {
            len: 2,
            positions: vec![5, 14],
        },
    ];
    assert_eq!(stitch(clusters, &spans, 3).len(), 2);
}

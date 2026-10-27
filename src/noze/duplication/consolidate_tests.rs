use super::*;
use crate::report::{CloneDetector, SemanticVerdict};

fn class(detector: CloneDetector, len: usize, occ: &[(&str, usize, usize)]) -> CloneClass {
    CloneClass {
        token_length: len,
        occurrences: occ
            .iter()
            .map(|&(file, start_row, end_row)| CloneOccurrence {
                file: file.into(),
                start_row,
                end_row,
            })
            .collect(),
        detector,
        ..Default::default()
    }
}

fn rows(class: &CloneClass) -> Vec<(String, usize, usize)> {
    class
        .occurrences
        .iter()
        .map(|o| (o.file.display().to_string(), o.start_row, o.end_row))
        .collect()
}

#[test]
fn longer_pair_inside_n_way_group_folds_and_widens() {
    let five_way = class(
        CloneDetector::Structural,
        73,
        &[
            ("full1", 4, 16),
            ("full2", 4, 16),
            ("part3", 3, 15),
            ("part4", 3, 15),
            ("part5", 3, 15),
        ],
    );
    let pair = class(
        CloneDetector::Structural,
        94,
        &[("full1", 1, 17), ("full2", 1, 17)],
    );
    let out = consolidate(vec![pair, five_way]);
    assert_eq!(out.len(), 1);
    assert_eq!(rows(&out[0])[0], ("full1".to_string(), 1, 17));
    assert_eq!(rows(&out[0])[2], ("part3".to_string(), 3, 15));
    assert_eq!(
        out[0].absorbed,
        vec![AbsorbedClone {
            detector: CloneDetector::Structural,
            token_length: 94,
            copies: 2,
        }]
    );
}

#[test]
fn near_miss_and_semantic_fold_into_structural() {
    let structural = class(CloneDetector::Structural, 52, &[("c", 6, 15), ("d", 6, 15)]);
    let mut near_miss = class(CloneDetector::NearMiss, 70, &[("c", 1, 16), ("d", 1, 16)]);
    near_miss.hint = Some("consistent-rename clone".into());
    let semantic = class(
        CloneDetector::Semantic {
            verdict: SemanticVerdict::EditedCopy,
        },
        77,
        &[("d", 3, 17), ("c", 3, 17)],
    );
    let out = consolidate(vec![semantic, near_miss, structural]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].detector, CloneDetector::Structural);
    assert_eq!(
        rows(&out[0]),
        vec![("c".into(), 1, 17), ("d".into(), 1, 17)]
    );
    assert_eq!(out[0].absorbed.len(), 2);
    assert_eq!(out[0].absorbed[0].detector, CloneDetector::NearMiss);
}

#[test]
fn non_overlapping_clones_between_same_files_stay_separate() {
    let first = class(CloneDetector::Structural, 60, &[("a", 1, 10), ("b", 1, 10)]);
    let second = class(
        CloneDetector::Structural,
        55,
        &[("a", 20, 30), ("b", 40, 50)],
    );
    assert_eq!(consolidate(vec![first, second]).len(), 2);
}

#[test]
fn pair_at_other_lines_than_n_way_group_stays_separate() {
    let three_way = class(
        CloneDetector::Structural,
        60,
        &[("a", 1, 10), ("b", 1, 10), ("c", 1, 10)],
    );
    let pair = class(
        CloneDetector::Structural,
        80,
        &[("a", 30, 40), ("b", 30, 40)],
    );
    assert_eq!(consolidate(vec![three_way, pair]).len(), 2);
}

#[test]
fn pair_with_one_copy_outside_n_way_group_stays_separate() {
    let three_way = class(
        CloneDetector::Structural,
        60,
        &[("a", 1, 10), ("b", 1, 10), ("c", 1, 10)],
    );
    let pair = class(CloneDetector::NearMiss, 80, &[("a", 1, 12), ("z", 1, 12)]);
    let out = consolidate(vec![three_way, pair]);
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|c| c.absorbed.is_empty()));
}

#[test]
fn two_copies_inside_one_host_occurrence_stay_separate() {
    let pair = class(
        CloneDetector::Structural,
        200,
        &[("a", 1, 100), ("b", 1, 100)],
    );
    let repeat = class(
        CloneDetector::Structural,
        50,
        &[("a", 10, 20), ("a", 50, 60)],
    );
    assert_eq!(consolidate(vec![pair, repeat]).len(), 2);
}

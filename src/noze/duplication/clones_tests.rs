use super::*;
use crate::spine::ir::FileId;
use crate::spine::parser::tokens::TokenSpan;

fn span(file_id: FileId) -> TokenSpan {
    TokenSpan {
        file_id,
        start_row: 1,
        end_row: 1,
        start_byte: 0,
        end_byte: 0,
    }
}

#[test]
fn groups_three_way_clone() {
    let text = vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 6, 1, 2, 3, 4, 7, 0];
    let spans = (0..text.len())
        .map(|i| span(FileId::from_index(i)))
        .collect();
    let mut master = Master { text, spans };
    let groups = extract(&mut master, 4, Nesting::Outermost);
    assert_eq!(groups.len(), 1, "one n-way group, not pairwise records");
    assert_eq!(groups[0].len, 4);
    assert_eq!(
        groups[0].positions.len(),
        3,
        "all three occurrences grouped"
    );
}

#[test]
fn nothing_below_threshold() {
    let text = vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 6, 0];
    let spans = (0..text.len())
        .map(|i| span(FileId::from_index(i)))
        .collect();
    let mut master = Master { text, spans };
    assert!(extract(&mut master, 5, Nesting::Outermost).is_empty());
}

#[test]
fn left_maximal_collapses_offset_fragments() {
    let text = vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 0];
    let spans = (0..text.len())
        .map(|i| span(FileId::from_index(i)))
        .collect();
    let mut master = Master { text, spans };
    let groups = extract(&mut master, 2, Nesting::Outermost);
    assert_eq!(
        groups.len(),
        1,
        "must not fragment into one group per offset"
    );
    assert_eq!(groups[0].len, 4);
    assert_eq!(groups[0].positions.len(), 2);
}

#[test]
fn adjacent_overlap_is_not_disjoint_despite_a_wide_span() {
    assert!(!can_be_disjoint(&[10, 0, 1], 4));
    assert!(can_be_disjoint(&[8, 0, 4], 4));
}

#[test]
fn nested_intervals_keep_longer_clones_among_fewer_occurrences() {
    let text = vec![1, 2, 3, 9, 1, 2, 3, 4, 5, 8, 1, 2, 3, 4, 5, 7, 0];
    let spans = (0..text.len())
        .map(|i| span(FileId::from_index(i)))
        .collect();
    let mut master = Master { text, spans };
    let shape = |groups: Vec<RawGroup>| -> Vec<(usize, usize)> {
        let mut out: Vec<_> = groups.iter().map(|g| (g.len, g.positions.len())).collect();
        out.sort_unstable();
        out
    };
    assert_eq!(
        shape(extract(&mut master, 3, Nesting::Outermost)),
        vec![(3, 3)]
    );
    assert_eq!(
        shape(extract(&mut master, 3, Nesting::Seeded { report_len: 3 })),
        vec![(3, 3), (5, 2)]
    );
}

use super::*;

sparse!(Vec<u8>);

#[test]
fn reads_see_an_empty_collection_without_allocating() {
    let rare: Rare<Vec<u8>> = Rare::default();
    assert!(rare.is_empty());
    assert!(rare.0.is_none());
}

#[test]
fn writes_allocate_and_compact_frees_an_empty_one() {
    let mut rare: Rare<Vec<u8>> = Rare::default();
    rare.push(1);
    assert_eq!(*rare, vec![1]);
    rare.clear();
    rare.compact();
    assert!(rare.0.is_none());
}

#[test]
fn serde_round_trip_keeps_items_and_drops_empty_boxes() {
    let full: Rare<Vec<u8>> = vec![3, 4].into();
    let json = serde_json::to_string(&full).unwrap();
    let back: Rare<Vec<u8>> = serde_json::from_str(&json).unwrap();
    assert_eq!(*back, vec![3, 4]);
    let empty: Rare<Vec<u8>> = serde_json::from_str("[]").unwrap();
    assert!(empty.0.is_none());
}

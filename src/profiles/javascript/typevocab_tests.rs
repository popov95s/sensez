use super::*;

#[test]
fn promise_of_void_is_void() {
    assert!(is_void("Promise<void>"));
    assert!(!is_void("Promise<Order>"));
}

#[test]
fn open_map_key_with_nested_generic_is_read_whole() {
    assert!(is_open_map("Record<Map<K, V>, Order>"));
    assert!(!is_open_map("Record<string, Map<K, V>>"));
}

#[test]
fn sets_and_records_mirror_python_loose_collections() {
    assert!(is_loose("Set<number>"));
    assert!(is_loose("Record<string, any>"));
    assert!(!is_loose("Set<UserDto>"));
    assert!(!is_loose("User[]"));
    assert!(!is_loose("string"));
}

#[test]
fn bool_and_dictish() {
    assert!(is_bool("boolean"));
    assert!(!is_bool("bool"));
    assert!(is_dictish("Record<string, number>"));
    assert!(!is_dictish("UserDto"));
}

#[test]
fn open_maps_have_typed_keys_or_primitive_values() {
    assert!(is_open_map(r#"Record<"a" | "b" | "c", number>"#));
    assert!(is_open_map("Record<Category, Item>"));
    assert!(is_open_map("Record<string, number>"));
    assert!(is_open_map("{ [k: string]: string }"));
    assert!(!is_open_map("Record<string, any>"));
    assert!(!is_open_map("{ [k: string]: Item }"));
    assert!(!is_open_map("{ host: string; port: number }"));
    assert!(!is_open_map("any"));
}

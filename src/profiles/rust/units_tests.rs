use super::RustProfile;
use crate::spine::parser::parse_source;

fn walked(src: &str) -> crate::spine::ir::Walked {
    parse_source(src.as_bytes(), 0, "m", &RustProfile).unwrap()
}

#[test]
fn extracts_function_units_and_type_hints() {
    let w = walked(
        r#"
fn decide(active: bool, dry_run: bool, tags: Vec<String>) -> (usize, String, bool) {
    (tags.len(), String::new(), active && dry_run)
}
"#,
    );
    let f = &w.units.functions[0];
    assert_eq!(f.name, "decide");
    assert_eq!(f.param_names, vec!["active", "dry_run", "tags"]);
    assert_eq!(f.max_tuple_return, 3);
    assert_eq!(
        w.units
            .type_hints
            .param_types
            .get(&("decide".into(), "active".into()))
            .map(String::as_str),
        Some("bool")
    );
    assert_eq!(
        w.units
            .type_hints
            .return_types
            .get("decide")
            .map(String::as_str),
        Some("(usize, String, bool)")
    );
}

#[test]
fn records_mutation_reassignment_and_implicit_schema_facts() {
    let w = walked(
        r#"
fn normalize(input: &mut Vec<String>, row: HashMap<String, String>) {
    let mut status = "new";
    status = "done";
    input.push(status.to_string());
    let _a = row["a"];
    let _b = row["b"];
    let _c = row["c"];
    let _d = row["d"];
}
"#,
    );
    let f = &w.units.functions[0];
    assert!(f.local_reassigns.contains_key("status"));
    assert!(f.mutated_names.contains("input"), "{:?}", f.mutated_names);
    assert_eq!(f.str_keys["row"].len(), 4);
}

#[test]
fn records_literal_membership_and_iteration_facts() {
    let w = walked(
        r#"
fn scan(items: &[String], code: &str) {
    if ["a", "b"].contains(&code) {}
    let _one = items.iter().filter(|x| !x.is_empty()).count();
    let _two = items.iter().map(|x| x.len()).sum::<usize>();
}
"#,
    );
    let f = &w.units.functions[0];
    assert_eq!(f.literal_membership_tests, 1);
    assert!(
        f.performance.iteration_calls.len() >= 2,
        "{:?}",
        f.performance.iteration_calls
    );
}

#[test]
fn records_nested_loop_and_sort_in_loop() {
    let w = walked(
        r#"
fn sort_pairs(groups: &mut [Vec<i32>]) {
    for group in groups {
        for value in group.iter() {
            let _ = value;
        }
        group.sort();
    }
}
"#,
    );
    let f = &w.units.functions[0];
    assert_eq!(f.performance.nested_loops.len(), 1);
    assert_eq!(f.performance.sorts_in_loops.len(), 1);
}

use super::super::PythonProfile;
use crate::profiles::test_fixtures::walked;

fn loop_declared(src: &str) -> Vec<String> {
    let w = walked(src, &PythonProfile);
    let frame = &w.units.functions[0].performance.loop_frames[0];
    frame.declared.iter().cloned().collect()
}

#[test]
fn reassigning_a_parameter_in_a_loop_is_not_a_declaration() {
    let src = "def grow(xs, acc):\n    for x in xs:\n        acc = acc + [x]\n        acc.sort()\n";
    assert!(loop_declared(src).is_empty());
}

#[test]
fn reassigning_an_earlier_local_in_a_loop_is_not_a_declaration() {
    let src = "def grow(xs):\n    acc = []\n    for x in xs:\n        acc = acc + [x]\n";
    assert!(loop_declared(src).is_empty());
}

#[test]
fn first_binding_in_a_loop_is_a_declaration() {
    let src =
        "def per_item(xs):\n    for x in xs:\n        local = list(x)\n        local.sort()\n";
    assert_eq!(loop_declared(src), vec!["local"]);
}

fn function(src: &str, name: &str) -> crate::spine::ir::FunctionUnit {
    walked(src, &PythonProfile)
        .units
        .functions
        .into_iter()
        .find(|f| f.name == name)
        .unwrap()
}

#[test]
fn init_is_a_constructor() {
    let src = "class A:\n    def __init__(self):\n        pass\n    def run(self):\n        pass\n";
    assert!(function(src, "__init__").is_constructor);
    assert!(!function(src, "run").is_constructor);
}

#[test]
fn assignments_to_self_attributes_count_as_self_writes() {
    let src = "class A:\n    def load(self, row):\n        self.a = row.a\n        self.b = row.b\n        x = row.c\n";
    assert_eq!(function(src, "load").self_writes, 2);
}

#[test]
fn nested_function_reading_an_enclosing_local_captures_its_parent() {
    let src = "def outer(limit):\n    def inner(x):\n        return x < limit\n    return inner\n";
    assert!(function(src, "inner").captures_parent);
}

#[test]
fn nested_function_using_only_its_own_names_does_not_capture() {
    let src =
        "def outer(limit):\n    def inner(x):\n        return x.limit(limit=1)\n    return inner\n";
    assert!(!function(src, "inner").captures_parent);
}

#[test]
fn helper_inside_a_pytest_test_has_a_test_callback_parent() {
    let src = "def test_rows():\n    def helper():\n        return 1\n    assert helper()\n";
    assert!(function(src, "helper").parent_is_test_callback);
}

#[test]
fn subscript_with_a_variable_key_is_a_dynamic_key_receiver() {
    let src = "def pick(row, key):\n    a = row[key]\n    b = row[\"name\"]\n    c = row[0]\n";
    let receivers = function(src, "pick").dynamic_key_receivers;
    assert_eq!(receivers.into_iter().collect::<Vec<_>>(), vec!["row"]);
}

#[test]
fn literal_keys_do_not_make_a_dynamic_key_receiver() {
    let src = "def pick(row):\n    a = row[\"name\"]\n    b = row[0]\n";
    assert!(function(src, "pick").dynamic_key_receivers.is_empty());
}

#[test]
fn repeated_primitive_tuple_annotation_is_homogeneous() {
    let src = "def span() -> tuple[int, int]:\n    return 1, 2\n\ndef pair() -> tuple[int, str]:\n    return 1, \"a\"\n";
    assert!(function(src, "span").homogeneous_tuple_return);
    assert!(!function(src, "pair").homogeneous_tuple_return);
}

#[test]
fn status_sibling_waives_only_compared_or_assigned_literals() {
    use crate::spine::ir::MagicContext;
    let src =
        "def check(res, status):\n    if res.status_code == 404:\n        retry(status, 503)\n";
    let contexts: Vec<_> = function(src, "check")
        .magic_numbers
        .into_iter()
        .map(|magic| (magic.text, magic.context))
        .collect();
    assert_eq!(
        contexts,
        vec![
            ("404".to_string(), MagicContext::HttpStatus),
            ("503".to_string(), MagicContext::Plain),
        ]
    );
}

#[test]
fn every_call_fact_records_its_start_byte() {
    let src = "def run(xs):\n    xs.sort()\n    xs.append(1)\n";
    let offsets: Vec<usize> = function(src, "run")
        .performance
        .calls
        .iter()
        .map(|call| call.offset)
        .collect();
    assert_eq!(offsets, vec![17, 31]);
}

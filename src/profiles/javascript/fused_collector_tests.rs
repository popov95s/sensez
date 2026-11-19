//! The fused collector's function facts.

use super::JsProfile;
use crate::spine::ir::FileId;
use crate::spine::parser::parse_source;

/// Parent metrics exclude nested arrow and declaration bodies.
#[test]
fn fused_collector_nested_arrows_and_declarations() {
    let src = b"function top(x) {\n  const f = () => x + 1;\n  function helper(y) { return y * 2; }\n  return f(helper(x));\n}\n";
    let functions = parse_source(src, FileId::default(), "m", &JsProfile)
        .unwrap()
        .units
        .functions;

    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "top");
    assert_eq!(functions[1].name, "f");
    assert_eq!(functions[2].name, "helper");
    assert!(functions[2].is_nested);
    assert_eq!(functions[2].parent, "top");

    assert_eq!(functions[0].return_count, 1);
    assert_eq!(functions[2].return_count, 1); // helper's own return
    assert_eq!(functions[0].branch_count, 0);
}

/// Nested loop in a conditional keeps cognitive, nesting and loop-call facts.
#[test]
fn fused_collector_nested_loop_and_conditional() {
    let src =
        b"function scan(items) {\n  if (items.length) {\n    for (const x of items) {\n      log(x);\n    }\n  }\n}\n";
    let functions = parse_source(src, FileId::default(), "m", &JsProfile)
        .unwrap()
        .units
        .functions;

    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    assert_eq!(f.max_nesting, 2);
    assert_eq!(f.cognitive, 3);
    assert_eq!(f.branch_count, 2);
    assert_eq!(f.performance.loop_calls.len(), 1);
    assert_eq!(f.performance.loops.len(), 1);
}

/// Class methods set is_method and self_attrs, in source order.
#[test]
fn fused_collector_class_method() {
    let src = b"class Store {\n  save(data) { this.buffer = data; return this.ok; }\n  load() { return this.buffer; }\n}\n";
    let units = parse_source(src, FileId::default(), "m", &JsProfile)
        .unwrap()
        .units;

    assert_eq!(units.classes.len(), 1);
    assert_eq!(units.classes[0].name, "Store");
    assert_eq!(units.classes[0].methods, vec!["save", "load"]);

    assert_eq!(units.functions.len(), 2);
    let save = &units.functions[0];
    assert_eq!(save.name, "save");
    assert!(save.is_method);
    assert_eq!(save.return_count, 1);
    assert!(save.self_attrs.contains("buffer"));
    assert!(save.self_attrs.contains("ok"));

    let load = &units.functions[1];
    assert_eq!(load.name, "load");
    assert!(load.is_method);
    assert_eq!(load.return_count, 1);
    assert!(load.self_attrs.contains("buffer"));
    assert!(save.receiver_access.contains_key("self"));
    assert!(load.receiver_access.contains_key("self"));
}

/// Parsing a fixture gives the same IR before and after traversal fusion.
#[test]
fn fused_collector_json_identity() {
    let src = b"function compute(n) {\n  let s = 0;\n  for (let i = 0; i < n; i++) {\n    if (i % 2) { s += i; } else { s += 1; }\n  }\n  return s;\n}\n";
    let functions = parse_source(src, FileId::default(), "m", &JsProfile)
        .unwrap()
        .units
        .functions;

    assert_eq!(functions.len(), 1);
    let f = &functions[0];
    assert_eq!(f.name, "compute");
    assert_eq!(f.max_nesting, 2);
    assert_eq!(f.return_count, 1);
    assert_eq!(f.branch_count, 2);
    assert_eq!(f.cognitive, 3);
    assert_eq!(f.magic_numbers.len(), 0);
    assert!(f.collapsible_nested_if_lines.is_empty());
    assert_eq!(f.comment_lines, 0);
    assert!(f.chain_hits.is_empty());
    assert!(!f.is_method);
    assert!(!f.is_nested);
    assert_eq!(f.parent, "");
    assert_eq!(f.param_names, vec!["n"]);
    assert_eq!(f.max_tuple_return, 0);
    assert!(f.local_reassigns.is_empty());
    assert!(f.receiver_access.is_empty());
    assert!(f.self_attrs.is_empty());
    assert!(f.own_method_calls.is_empty());
    assert!(f.str_keys.is_empty());
    assert!(f.schema_calls.is_empty());
    assert!(f.validated_names.is_empty());
    assert!(f.returned_constructors.is_empty());
    assert!(f.mutated_names.is_empty());
    assert!(f.attr_mutated_names.is_empty());
    assert_eq!(f.performance.loops.len(), 1);
    assert_eq!(f.performance.loops[0].line, 3);
    assert!(f.performance.nested_loops.is_empty());
    assert!(f.performance.calls.is_empty());
    assert!(f.performance.loop_calls.is_empty());
    assert!(f.performance.iteration_calls.is_empty());
    assert!(f.performance.sorts_in_loops.is_empty());
    assert_eq!(f.review_risks.broad_handlers, 0);
    assert_eq!(f.review_risks.empty_fallbacks, 0);
    assert_eq!(f.review_risks.repeated_guards, 0);
    assert!(f.literal_memberships.is_empty());
    assert!(f.string_fallbacks.is_empty());
}

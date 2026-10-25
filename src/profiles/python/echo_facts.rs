//! Assertion operands for `fixture_echo_assertion`.

use crate::profiles::echo_assertion::{echo_shape, Alias, EchoSyntax};
use crate::spine::ir::{EchoAssertion, EchoShape, FunctionUnit};
use tree_sitter::Node;

const SYNTAX: EchoSyntax = EchoSyntax {
    literal_kinds: &["integer", "float", "string", "true", "false", "none"],
    volatile_kinds: &["call", "await"],
    member_kind: "attribute",
    object_field: "object",
    property_field: "attribute",
    block_kinds: &["block", "module"],
    comment_kind: "comment",
    alias,
};

const EQUALITY_OPERATORS: &[&str] = &["==", "is"];
const EQUALITY_ASSERTS: &[&str] = &[
    "self.assertEqual",
    "self.assertEquals",
    "self.assertIs",
    "self.assertListEqual",
    "self.assertDictEqual",
    "self.assertTupleEqual",
    "self.assertSetEqual",
    "self.assertSequenceEqual",
    "self.assertCountEqual",
];
const TRUTH_ASSERTS: &[&str] = &["self.assertTrue"];

pub(super) fn scan(unit: &mut FunctionUnit, node: Node<'_>, src: &[u8]) {
    let shape = match node.kind() {
        "assert_statement" => node.named_child(0).and_then(|test| asserted(test, src)),
        "call" => unittest_assert(node, src),
        _ => None,
    };
    if let Some(shape) = shape {
        unit.review_risks.echo_assertions.push(EchoAssertion {
            line: node.start_position().row + 1,
            shape,
        });
    }
}

/// `assert True`, or `assert a == a`.
fn asserted(test: Node<'_>, src: &[u8]) -> Option<EchoShape> {
    if test.kind() == "true" {
        return Some(EchoShape::Literals);
    }
    if test.kind() != "comparison_operator" || test.child_count() != 3 {
        return None;
    }
    let operator = test.child(1)?.kind();
    EQUALITY_OPERATORS
        .contains(&operator)
        .then(|| echo_shape(test.named_child(0)?, test.named_child(1)?, src, &SYNTAX))
        .flatten()
}

/// `self.assertEqual(a, a)` or `self.assertTrue(True)`.
fn unittest_assert(call: Node<'_>, src: &[u8]) -> Option<EchoShape> {
    let callee = call.child_by_field_name("function")?.utf8_text(src).ok()?;
    let args = positional(call);
    if TRUTH_ASSERTS.contains(&callee) {
        return args
            .first()
            .filter(|arg| arg.kind() == "true")
            .map(|_| EchoShape::Literals);
    }
    let is_equality = EQUALITY_ASSERTS.contains(&callee);
    match args.as_slice() {
        [actual, expected, ..] if is_equality => echo_shape(*actual, *expected, src, &SYNTAX),
        _ => None,
    }
}

fn positional(call: Node<'_>) -> Vec<Node<'_>> {
    let Some(args) = call.child_by_field_name("arguments") else {
        return Vec::new();
    };
    let mut cursor = args.walk();
    let nodes = args
        .named_children(&mut cursor)
        .filter(|arg| !matches!(arg.kind(), "keyword_argument" | "comment"))
        .collect();
    nodes
}

/// `result = fixture` binds `result` to `fixture` unchanged.
fn alias(statement: Node<'_>, src: &[u8]) -> Option<Alias> {
    let assignment = statement
        .named_child(0)
        .filter(|_| statement.kind() == "expression_statement")?;
    let name = assignment.child_by_field_name("left")?;
    let value = assignment.child_by_field_name("right")?;
    (assignment.kind() == "assignment"
        && name.kind() == "identifier"
        && value.kind() == "identifier")
        .then(|| Alias {
            name: name.utf8_text(src).unwrap_or_default().to_string(),
            source: value.utf8_text(src).unwrap_or_default().to_string(),
        })
}

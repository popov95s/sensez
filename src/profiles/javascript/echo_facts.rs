//! Assertion operands and mock usage for `fixture_echo_assertion`.

use crate::profiles::echo_assertion::{echo_shape, Alias, EchoSyntax};
use crate::spine::ir::{EchoAssertion, FunctionUnit};
use tree_sitter::Node;

const SYNTAX: EchoSyntax = EchoSyntax {
    literal_kinds: &["number", "string", "true", "false", "null", "undefined"],
    volatile_kinds: &["call_expression", "new_expression", "await_expression"],
    member_kind: "member_expression",
    object_field: "object",
    property_field: "property",
    block_kinds: &["statement_block", "program"],
    comment_kind: "comment",
    alias,
};

const EQUALITY_MATCHERS: &[&str] = &["toBe", "toEqual", "toStrictEqual"];
const EQUALITY_ASSERTS: &[&str] = &[
    "assert.equal",
    "assert.strictEqual",
    "assert.deepEqual",
    "assert.deepStrictEqual",
];
/// Was-called matchers that check neither arguments nor call counts.
const CALLED_MATCHERS: &[&str] = &["toHaveBeenCalled", "toBeCalled"];
const MOCK_FACTORIES: &[&str] = &["vi.fn", "jest.fn", "vi.spyOn", "jest.spyOn"];
const MOCK_WRAPPERS: &[&str] = &["vi.mocked", "jest.mocked"];
const MOCK_PROGRAMMING: &[&str] = &[
    "mockReturnValue",
    "mockResolvedValue",
    "mockRejectedValue",
    "mockImplementation",
];

pub(super) fn scan(unit: &mut FunctionUnit, node: Node<'_>, src: &[u8]) {
    match node.kind() {
        "call_expression" => scan_call(unit, node, src),
        "variable_declarator" => {
            if let Some(name) = programmed_mock_declaration(node, src) {
                unit.review_risks.programmed_mocks.push(name.to_string());
            }
        }
        _ => {}
    }
}

fn scan_call(unit: &mut FunctionUnit, call: Node<'_>, src: &[u8]) {
    let Some(callee) = call.child_by_field_name("function") else {
        return;
    };
    let args = arguments(call);
    if callee.kind() != "member_expression" {
        return;
    }
    let text = callee.utf8_text(src).unwrap_or_default();
    if EQUALITY_ASSERTS.contains(&text) && args.len() >= 2 {
        record_echo(unit, call, args[0], args[1], src);
        return;
    }
    let (Some(object), Some(member)) = (
        callee.child_by_field_name("object"),
        callee
            .child_by_field_name("property")
            .and_then(|property| property.utf8_text(src).ok()),
    ) else {
        return;
    };
    if MOCK_PROGRAMMING
        .iter()
        .any(|prefix| member.starts_with(prefix))
    {
        if let Ok(receiver) = unwrap_mocked(object, src).utf8_text(src) {
            unit.review_risks
                .programmed_mocks
                .push(receiver.to_string());
        }
        return;
    }
    let subject = expect_subject(object, src);
    if let Some(subject) =
        subject.filter(|_| EQUALITY_MATCHERS.contains(&member) && args.len() == 1)
    {
        record_echo(unit, call, subject, args[0], src);
    }
    if !CALLED_MATCHERS.contains(&member) {
        return;
    }
    if let Some(subject) = subject {
        let receiver = subject.utf8_text(src).unwrap_or_default();
        unit.review_risks
            .mock_call_checks
            .push(receiver.to_string());
    }
}

fn record_echo(
    unit: &mut FunctionUnit,
    call: Node<'_>,
    actual: Node<'_>,
    expected: Node<'_>,
    src: &[u8],
) {
    if let Some(shape) = echo_shape(actual, expected, src, &SYNTAX) {
        unit.review_risks.echo_assertions.push(EchoAssertion {
            line: call.start_position().row + 1,
            shape,
        });
    }
}

/// `x` for `vi.mocked(x)` / `jest.mocked(x)`, else `node`.
fn unwrap_mocked<'a>(node: Node<'a>, src: &[u8]) -> Node<'a> {
    let is_mocked = node
        .child_by_field_name("function")
        .and_then(|callee| callee.utf8_text(src).ok())
        .is_some_and(|callee| MOCK_WRAPPERS.contains(&callee));
    match arguments(node).first() {
        Some(inner) if is_mocked => *inner,
        _ => node,
    }
}

/// The single argument of a bare `expect(x)` call.
fn expect_subject<'a>(node: Node<'a>, src: &[u8]) -> Option<Node<'a>> {
    if node.kind() != "call_expression" {
        return None;
    }
    let callee = node.child_by_field_name("function")?;
    let args = arguments(node);
    (callee.utf8_text(src).ok()? == "expect" && args.len() == 1).then(|| args[0])
}

fn arguments(call: Node<'_>) -> Vec<Node<'_>> {
    let Some(args) = call.child_by_field_name("arguments") else {
        return Vec::new();
    };
    let mut cursor = args.walk();
    let nodes = args
        .named_children(&mut cursor)
        .filter(|arg| arg.kind() != "comment")
        .collect();
    nodes
}

/// `const fn = vi.fn().mockReturnValue(x)` binds `fn` to a programmed mock.
fn programmed_mock_declaration<'a>(declarator: Node<'_>, src: &'a [u8]) -> Option<&'a str> {
    let name = declarator.child_by_field_name("name")?;
    let value = declarator.child_by_field_name("value")?;
    let callee = value.child_by_field_name("function").filter(|callee| {
        value.kind() == "call_expression" && callee.kind() == "member_expression"
    })?;
    let member = callee
        .child_by_field_name("property")?
        .utf8_text(src)
        .ok()?;
    let factory = callee
        .child_by_field_name("object")?
        .child_by_field_name("function")?
        .utf8_text(src)
        .ok()?;
    let programmed = MOCK_FACTORIES.contains(&factory)
        && MOCK_PROGRAMMING
            .iter()
            .any(|prefix| member.starts_with(prefix));
    (name.kind() == "identifier" && programmed)
        .then(|| name.utf8_text(src).ok())
        .flatten()
}

/// `const result = input;` binds `result` to `input` unchanged.
fn alias(statement: Node<'_>, src: &[u8]) -> Option<Alias> {
    if !matches!(
        statement.kind(),
        "lexical_declaration" | "variable_declaration"
    ) {
        return None;
    }
    let declarator = statement.named_child(0)?;
    let name = declarator.child_by_field_name("name")?;
    let value = declarator.child_by_field_name("value")?;
    (name.kind() == "identifier" && value.kind() == "identifier").then(|| Alias {
        name: name.utf8_text(src).unwrap_or_default().to_string(),
        source: value.utf8_text(src).unwrap_or_default().to_string(),
    })
}

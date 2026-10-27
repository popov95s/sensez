//! Doubles installed with `patch`, `mocker.patch`, and `monkeypatch.setattr`.

use super::symbols::string_value;
use crate::spine::ir::{MockFidelity, MockHost, MockScope, MockTarget};
use tree_sitter::Node;

const PATH_PATCHES: &[&str] = &[
    "patch",
    "mock.patch",
    "unittest.mock.patch",
    "mocker.patch",
    "monkeypatch.setattr",
];
const OBJECT_PATCHES: &[&str] = &[
    "patch.object",
    "mock.patch.object",
    "unittest.mock.patch.object",
    "mocker.patch.object",
    "monkeypatch.setattr",
];

/// Record `call` when it patches a dotted path or an attribute of a named object.
pub(super) fn record(call: Node, src: &[u8], out: &mut Vec<MockTarget>) {
    let Some(callee) = call
        .child_by_field_name("function")
        .and_then(|callee| callee.utf8_text(src).ok())
    else {
        return;
    };
    let Some(args) = call.child_by_field_name("arguments") else {
        return;
    };
    let mut cursor = args.walk();
    let positional: Vec<Node> = args
        .named_children(&mut cursor)
        .filter(|arg| arg.kind() != "keyword_argument")
        .collect();
    let target = match positional.as_slice() {
        [path, ..] if path.kind() == "string" && PATH_PATCHES.contains(&callee) => {
            dotted_target(&string_value(*path, src).unwrap_or_default())
        }
        [object, member, ..]
            if object.kind() == "identifier"
                && member.kind() == "string"
                && OBJECT_PATCHES.contains(&callee) =>
        {
            object
                .utf8_text(src)
                .ok()
                .zip(string_value(*member, src))
                .map(|(object, member)| {
                    (
                        MockHost::Binding(object.to_string()),
                        MockScope::Member(member),
                    )
                })
        }
        _ => None,
    };
    let Some((host, scope)) = target else {
        return;
    };
    out.push(MockTarget {
        line: call.start_position().row + 1,
        host,
        scope,
        fidelity: fidelity(args, src),
    });
}

fn dotted_target(path: &str) -> Option<(MockHost, MockScope)> {
    let (module, member) = path.rsplit_once('.')?;
    Some((
        MockHost::Specifier(module.to_string()),
        MockScope::Member(member.to_string()),
    ))
}

/// `wraps=` keeps the real callable behind the double.
fn fidelity(args: Node, src: &[u8]) -> MockFidelity {
    let mut cursor = args.walk();
    let wraps = args.named_children(&mut cursor).any(|arg| {
        arg.kind() == "keyword_argument"
            && arg
                .child_by_field_name("name")
                .and_then(|name| name.utf8_text(src).ok())
                == Some("wraps")
    });
    if wraps {
        MockFidelity::KeepsOriginal
    } else {
        MockFidelity::Replaces
    }
}

//! Module doubles installed with `vi.mock` / `jest.mock`.

use crate::profiles::node_names::NodeNames;
use crate::spine::ir::{MockFidelity, MockHost, MockScope, MockTarget};
use tree_sitter::Node;

const MODULE_MOCKS: &[&str] = &["vi.mock", "vi.doMock", "jest.mock", "jest.doMock"];
const KEEPS_ORIGINAL: &[&str] = &["importActual", "requireActual", "spy: true"];

/// Record `call` when it mocks a module named by a string literal.
pub(super) fn record(call: Node, src: &[u8], out: &mut Vec<MockTarget>) {
    let is_module_mock = call
        .field("function")
        .and_then(|callee| callee.utf8_text(src).ok())
        .is_some_and(|callee| MODULE_MOCKS.contains(&callee));
    let Some(args) = call.field("arguments").filter(|_| is_module_mock) else {
        return;
    };
    let Some(specifier) = args
        .named_child(0)
        .filter(|arg| arg.kind_name() == "string")
    else {
        return;
    };
    let Ok(text) = specifier.utf8_text(src) else {
        return;
    };
    let rest = String::from_utf8_lossy(&src[specifier.end_byte()..args.end_byte()]);
    out.push(MockTarget {
        line: call.start_position().row + 1,
        host: MockHost::Specifier(text.trim_matches(['"', '\'']).to_string()),
        scope: MockScope::WholeModule,
        fidelity: if receives_original(args)
            || KEEPS_ORIGINAL.iter().any(|marker| rest.contains(marker))
        {
            MockFidelity::KeepsOriginal
        } else {
            MockFidelity::Replaces
        },
    });
}

/// A factory that takes a parameter receives the real module (`async (importOriginal) => ...`).
fn receives_original(args: Node) -> bool {
    args.named_child(1)
        .filter(|factory| {
            matches!(
                factory.kind_name(),
                "arrow_function" | "function_expression"
            )
        })
        .is_some_and(|factory| {
            factory.field("parameter").is_some()
                || factory
                    .field("parameters")
                    .is_some_and(|params| params.named_child_count() > 0)
        })
}

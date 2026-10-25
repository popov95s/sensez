//! JS/TS writes to members, outer names and the network.

use super::units::is_function;
use crate::profiles::lazy_init::is_guarded_by;
use crate::spine::ir::{FunctionUnit, StateWrite, StateWriteKind};
use tree_sitter::Node;

const LAZY_OPERATORS: &[&str] = &["??=", "||="];
const READ_REQUEST_METHODS: &[&str] = &["GET", "HEAD", "OPTIONS"];

/// Record the write `node` performs, when it is one.
pub fn scan(unit: &mut FunctionUnit, node: Node, src: &[u8]) {
    let write = match node.kind() {
        "assignment_expression" | "augmented_assignment_expression" => node
            .child_by_field_name("left")
            .and_then(|left| assignment(node, left, src)),
        "update_expression" => node
            .child_by_field_name("argument")
            .and_then(|target| assignment(node, target, src)),
        "call_expression" => request(node, src),
        _ => None,
    };
    unit.state_writes.extend(write);
}

/// Drop plain assignments to names this function declares or takes as parameters.
pub fn drop_locals(unit: &mut FunctionUnit) {
    let bindings = &unit.performance.bindings;
    let params = &unit.param_names;
    unit.state_writes.retain(|write| {
        let name = write.target.as_str();
        let rebinds = bindings.iter().any(|(bound, _)| bound == name)
            || params.iter().any(|param| param == name);
        write.kind == StateWriteKind::Request || !rebinds
    });
}

fn assignment(node: Node, target: Node, src: &[u8]) -> Option<StateWrite> {
    let text = target.utf8_text(src).ok()?;
    let root = chain_root(target)?;
    let own = root.kind() == "this";
    let key = match own {
        true => target
            .child_by_field_name("property")
            .and_then(|property| property.utf8_text(src).ok())?,
        false => root.utf8_text(src).ok()?,
    };
    let lazy = node
        .child_by_field_name("operator")
        .and_then(|operator| operator.utf8_text(src).ok())
        .is_some_and(|operator| LAZY_OPERATORS.contains(&operator))
        || is_guarded_by(
            node,
            src,
            key,
            "if_statement",
            is_function,
            super::wording::WORDING.identifier_extra_chars,
        );
    let kind = match (lazy, own) {
        (true, _) => StateWriteKind::LazyInit,
        (false, true) => StateWriteKind::OwnMember,
        (false, false) => StateWriteKind::OuterName,
    };
    Some(StateWrite {
        line: node.start_position().row + 1,
        target: text.to_string(),
        kind,
    })
}

/// `this` or the identifier a member/subscript chain starts from.
fn chain_root(node: Node) -> Option<Node> {
    match node.kind() {
        "identifier" | "this" => Some(node),
        "member_expression" | "subscript_expression" => {
            chain_root(node.child_by_field_name("object")?)
        }
        _ => None,
    }
}

/// `fetch(url, { method: "POST" })`.
fn request(call: Node, src: &[u8]) -> Option<StateWrite> {
    let callee = call.child_by_field_name("function")?;
    if callee.utf8_text(src).ok()? != "fetch" {
        return None;
    }
    let options = call.child_by_field_name("arguments")?.named_child(1)?;
    let mut cursor = options.walk();
    let method = options
        .named_children(&mut cursor)
        .filter(|pair| pair.kind() == "pair")
        .find(|pair| {
            pair.child_by_field_name("key")
                .and_then(|key| key.utf8_text(src).ok())
                == Some("method")
        })?
        .child_by_field_name("value")
        .filter(|value| value.kind() == "string")?
        .utf8_text(src)
        .ok()?
        .trim_matches(['"', '\'', '`'])
        .to_ascii_uppercase();
    (!READ_REQUEST_METHODS.contains(&method.as_str())).then(|| StateWrite {
        line: call.start_position().row + 1,
        target: format!("fetch {method}"),
        kind: StateWriteKind::Request,
    })
}

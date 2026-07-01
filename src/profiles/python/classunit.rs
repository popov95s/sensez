//! Python class-level field extraction.

use crate::spine::ir::ClassProperty;
use tree_sitter::Node;

pub fn properties(class: Node, src: &[u8]) -> Vec<ClassProperty> {
    let Some(body) = class.child_by_field_name("body") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut cursor = body.walk();
    for member in body.named_children(&mut cursor) {
        if let Some(prop) = property_from_member(member, src) {
            out.push(prop);
        }
    }
    out
}

fn property_from_member(member: Node, src: &[u8]) -> Option<ClassProperty> {
    let assignment = if member.kind() == "assignment" {
        member
    } else if member.kind() == "expression_statement" {
        first_child_kind(member, "assignment")?
    } else {
        return None;
    };
    let left = assignment.child_by_field_name("left")?;
    let name = left
        .utf8_text(src)
        .ok()
        .filter(|name| name.chars().all(|c| c == '_' || c.is_alphanumeric()))?;
    let initializer_type =
        super::typehints::instantiated_type(assignment.child_by_field_name("right"), src);
    let ty = super::typehints::type_text(assignment.child_by_field_name("type"), src)
        .or_else(|| initializer_type.clone())?;
    Some(ClassProperty {
        name: name.to_string(),
        type_name: normalize_type(&ty),
        initializer_type,
        line: assignment.start_position().row + 1,
    })
}

fn first_child_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == kind);
    found
}

fn normalize_type(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

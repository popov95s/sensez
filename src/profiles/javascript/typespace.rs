//! TS type positions for duplication lexemes: inline annotations are dropped so
//! a typed copy matches its JS twin; declared types get their own code space.

use crate::profiles::lexeme as shared;
use tree_sitter::Node;

/// Where a node sits relative to TS type syntax.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TypeSpace {
    Value,
    Annotation,
    Declaration,
}

impl TypeSpace {
    /// The space of `child`, entered from `parent` while in `self`.
    pub fn enter(self, parent: Node, child: Node) -> Self {
        if self == Self::Declaration || is_type_declaration(child.kind()) {
            Self::Declaration
        } else if self == Self::Annotation || is_inline_annotation(parent, child) {
            Self::Annotation
        } else {
            Self::Value
        }
    }

    /// The lexeme code to emit here, or `None` when annotations are dropped.
    pub fn code(self, code: u64) -> Option<u64> {
        match self {
            Self::Value => Some(code),
            Self::Annotation => None,
            Self::Declaration => Some(shared::type_position(code)),
        }
    }
}

fn is_type_declaration(kind: &str) -> bool {
    matches!(
        kind,
        "interface_declaration"
            | "type_alias_declaration"
            | "function_signature"
            | "abstract_method_signature"
            | "index_signature"
            | "ambient_declaration"
    )
}

fn is_inline_annotation(parent: Node, child: Node) -> bool {
    match parent.kind() {
        "as_expression" | "satisfies_expression" => parent.named_child(0) != Some(child),
        _ => matches!(
            child.kind(),
            "type_annotation"
                | "opting_type_annotation"
                | "omitting_type_annotation"
                | "adding_type_annotation"
                | "asserts_annotation"
                | "type_predicate_annotation"
                | "type_arguments"
                | "type_parameters"
                | "implements_clause"
        ),
    }
}

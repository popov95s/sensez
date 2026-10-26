//! Attribute-access chain shape.

use crate::profiles::walk::ChainShape;

pub(super) const CHAIN: ChainShape = ChainShape {
    member_kind: "attribute",
    object_field: "object",
    root_kinds: &["identifier"],
};

//! Field-access chain shape.

use crate::profiles::walk::ChainShape;

pub(super) const CHAIN: ChainShape = ChainShape {
    member_kind: "field_expression",
    object_field: "value",
    root_kinds: &["identifier", "self"],
};

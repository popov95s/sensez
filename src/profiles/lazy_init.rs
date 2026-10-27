//! Assignments guarded by a check of the same target (`if (!x) x = …`).

use crate::profiles::lineage::Lineage;
use crate::profiles::node_names::NodeNames;
use crate::profiles::typevocab::is_ident_char;

/// `node` sits under an `if_kind` node whose condition names `key`, inside the same function.
pub(crate) fn is_guarded_by(
    lineage: Lineage,
    src: &[u8],
    key: &str,
    if_kind: &str,
    is_function: fn(&str) -> bool,
    identifier_chars: &[char],
) -> bool {
    let mut current = lineage.node();
    while let Some(parent) = lineage.parent(current) {
        if is_function(parent.kind_name()) {
            return false;
        }
        let names_key = parent
            .field("condition")
            .filter(|_| parent.kind_name() == if_kind)
            .and_then(|condition| condition.utf8_text(src).ok())
            .is_some_and(|text| mentions(text, key, identifier_chars));
        if names_key {
            return true;
        }
        current = parent;
    }
    false
}

fn mentions(text: &str, key: &str, identifier_chars: &[char]) -> bool {
    text.split(|ch: char| !is_ident_char(ch, identifier_chars))
        .any(|word| word == key)
}

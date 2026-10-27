//! Shared test helpers: parse a source with one profile.

use crate::profiles::ParseProfile;
use crate::spine::ir::Walked;
use crate::spine::parser::{parse_source, StructuralToken};

pub(crate) fn walked(src: impl AsRef<[u8]>, profile: &dyn ParseProfile) -> Walked {
    parse_source(src.as_ref(), 0, "m", profile).unwrap()
}

pub(crate) fn tokens(src: impl AsRef<[u8]>, profile: &dyn ParseProfile) -> Vec<StructuralToken> {
    walked(src, profile).syntax.tokens
}

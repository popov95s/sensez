use super::hub;
use crate::source_state::SourceState;
use std::path::Path;

pub(crate) fn observe_source_state(root: &Path, state: SourceState) {
    hub::observe_source_state(root, state);
}

pub fn recapture() {
    super::recapture::run();
}

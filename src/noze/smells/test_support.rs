//! Shared fixtures for smell detector tests.

use super::detect_local;
use crate::config::smells::Smells;
use crate::report::{SmellFinding, SmellKind};
use crate::spine::parser::{parse_file, ParsedFile};

/// `body` parsed as a file called `name` in a throwaway directory.
pub(crate) fn parsed(name: &str, body: &str) -> ParsedFile {
    let tmp = tempfile::tempdir().unwrap();
    parse_file(&crate::test_support::write(tmp.path(), name, body), 0).unwrap()
}

/// File-local findings for `body` parsed as `name`.
pub(super) fn local(name: &str, body: &str, cfg: &Smells) -> Vec<SmellFinding> {
    detect_local(&parsed(name, body), cfg)
}

/// File-local findings of one kind.
pub(super) fn local_of(name: &str, body: &str, cfg: &Smells, kind: SmellKind) -> Vec<SmellFinding> {
    local(name, body, cfg)
        .into_iter()
        .filter(|finding| finding.kind == kind)
        .collect()
}

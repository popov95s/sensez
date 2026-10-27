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

/// Every file of a throwaway project and the import graph between them.
pub(super) fn project(
    files: &[(&str, &str)],
) -> (Vec<ParsedFile>, crate::spine::graph::CodebaseGraph) {
    let tmp = tempfile::tempdir().unwrap();
    let parsed: Vec<ParsedFile> = files
        .iter()
        .enumerate()
        .map(|(index, (name, body))| {
            parse_file(
                &crate::test_support::write(tmp.path(), name, body),
                index as u32,
            )
            .unwrap()
        })
        .collect();
    let graph = crate::spine::graph::build(&parsed, &[tmp.path().to_path_buf()]);
    (parsed, graph)
}

use std::path::Path;

/// A file offered to a manifest's entry-point lookup.
pub struct EntryCandidate<'a> {
    pub path: &'a Path,
    pub runs_as_program: bool,
}

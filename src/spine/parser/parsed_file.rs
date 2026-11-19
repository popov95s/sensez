use crate::report::ScanIssue;
use crate::spine::cache::SourceFingerprint;
use crate::spine::ir::{Language, Walked};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug)]
pub struct ParsedFile {
    pub path: PathBuf,
    pub language: Language,
    pub lines: u32,
    pub fingerprint: SourceFingerprint,
    pub walked: Arc<Walked>,
    /// Machine-written source: kept for the graph, skipped by smells and duplication.
    pub is_generated: bool,
    pub runs_as_program: bool,
}

#[derive(Debug, Default)]
pub struct ParseBatch {
    pub files: Vec<ParsedFile>,
    pub issues: Vec<ScanIssue>,
}

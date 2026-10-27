//! What to do about a dead-code finding, by its kind and reason.

use super::DeadCodeFinding;
use crate::spine::ir::SymbolKind;

impl DeadCodeFinding {
    /// One-line next step for the reader.
    pub fn advice(&self) -> &'static str {
        match (self.kind, self.reason.as_str()) {
            (SymbolKind::Module, "test_only") => {
                "only tests import this module: delete it with its tests, or list it under dead_code.test_support"
            }
            (SymbolKind::Module, _) => {
                "nothing imports this module: delete it, or add it to dead_code.entry_globs if a tool runs it"
            }
            (_, "test_only") => "only tests use it: delete it with its tests, or stop exporting it",
            (SymbolKind::ReExport, _) => {
                "nothing imports it through this barrel or uses it elsewhere: remove the re-export and its definition"
            }
            _ => "nothing uses it: delete it, or add it to dead_code.entrypoint_names if it is called dynamically",
        }
    }
}

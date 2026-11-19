use super::language_info::LanguageInfo;
use crate::spine::ir::{FileId, Language, Walked};
use std::path::Path;

pub trait ParseProfile: Send + Sync {
    fn info(&self) -> &'static LanguageInfo;
    fn ts_language(&self) -> tree_sitter::Language;
    fn walk(
        &self,
        root: tree_sitter::Node,
        src: &[u8],
        file_id: FileId,
        module_name: &str,
    ) -> Walked;
    fn is_generated_or_data_source(&self, _path: &Path) -> bool {
        false
    }
    fn is_declaration_only(&self, _path: &Path) -> bool {
        false
    }
    /// Parsed but machine-written: kept in the graph, skipped by smells and duplication.
    fn is_generated(&self, _path: &Path, _src: &[u8]) -> bool {
        false
    }
    /// Source that runs as a program on its own (shebang, main guard), so no importer is needed.
    fn runs_as_program(&self, _src: &[u8]) -> bool {
        false
    }
    /// Languages sharing a clone family are compared as one duplication cohort.
    fn clone_family(&self) -> Language {
        self.info().language
    }
}

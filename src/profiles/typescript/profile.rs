use super::info::{TSX_INFO, TS_INFO};
use crate::profiles::javascript::{declaration, generated, programs, traversal};
use crate::profiles::{LanguageInfo, ParseProfile};
use crate::spine::ir::{FileId, Language, Walked};
use std::path::Path;

/// The TypeScript language profile.
pub struct TsProfile;

impl ParseProfile for TsProfile {
    fn info(&self) -> &'static LanguageInfo {
        &TS_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn walk(
        &self,
        root: tree_sitter::Node,
        src: &[u8],
        file_id: FileId,
        module_name: &str,
    ) -> Walked {
        traversal::walk(root, src, file_id, module_name)
    }

    fn is_declaration_only(&self, path: &Path) -> bool {
        declaration::is_declaration_file(path)
    }

    fn is_generated(&self, path: &Path, src: &[u8]) -> bool {
        generated::is_generated(path, src)
    }

    fn runs_as_program(&self, src: &[u8]) -> bool {
        programs::is_program(src)
    }

    fn clone_family(&self) -> Language {
        Language::JavaScript
    }
}

/// The TSX language profile.
pub struct TsxProfile;

impl ParseProfile for TsxProfile {
    fn info(&self) -> &'static LanguageInfo {
        &TSX_INFO
    }

    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    }

    fn walk(
        &self,
        root: tree_sitter::Node,
        src: &[u8],
        file_id: FileId,
        module_name: &str,
    ) -> Walked {
        traversal::walk(root, src, file_id, module_name)
    }

    fn is_generated(&self, path: &Path, src: &[u8]) -> bool {
        generated::is_generated(path, src)
    }

    fn runs_as_program(&self, src: &[u8]) -> bool {
        programs::is_program(src)
    }

    fn clone_family(&self) -> Language {
        Language::JavaScript
    }
}

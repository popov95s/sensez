use crate::profiles::wording::SmellWording;
use crate::profiles::{registry, TypeVocabularyProfile};
use crate::spine::ir::{Language, TypeHints};
use crate::spine::parser::ParsedFile;
use rustc_hash::FxHashSet;
use std::path::Path;

pub(in crate::noze) struct SmellContext<'a> {
    pub path: &'a Path,
    pub language: Language,
    pub type_hints: &'a TypeHints,
    pub type_vocabulary: &'static dyn TypeVocabularyProfile,
    pub wording: &'static SmellWording,
    pub test_source: bool,
    /// Local names bound by imports from generated modules; chains from them are API navigation.
    pub generated_roots: FxHashSet<&'a str>,
}

impl<'a> SmellContext<'a> {
    pub fn from_file(file: &'a ParsedFile) -> Self {
        Self {
            path: &file.path,
            language: file.language,
            type_hints: &file.walked.units.type_hints,
            type_vocabulary: registry::type_vocabulary(file.language),
            wording: registry::smell_wording(file.language),
            test_source: registry::is_test_source(file.language, &file.path),
            generated_roots: file
                .walked
                .symbols
                .imports
                .iter()
                .filter(|import| {
                    registry::is_generated_import(file.language, &import.target_module)
                })
                .flat_map(|import| import.bindings.iter().map(String::as_str))
                .collect(),
        }
    }
}

use super::dead_code_defaults::DeadCodeDefaults;
use super::decorator_class::DecoratorClass;
use super::entry_candidate::EntryCandidate;
use crate::spine::ir::{ClassProperty, ClassUnit};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub trait DeadCodeProfile: Send + Sync {
    fn classify_decorator(
        &self,
        paths: Option<&Vec<String>>,
        user_entrypoints: &HashSet<String>,
    ) -> DecoratorClass;
    /// True when `symbol` is uninteresting dead code by language convention and is skipped.
    fn is_conventionally_private(&self, symbol: &str) -> bool;
    fn is_entry_file_stem(&self, stem: &str) -> bool;
    fn manages_class_properties(
        &self,
        _class: &ClassUnit,
        _decorators: Option<&Vec<String>>,
    ) -> bool {
        false
    }
    fn manages_property(&self, _property: &ClassProperty) -> bool {
        false
    }
    fn requires_property_usage_evidence(&self, _class: &ClassUnit) -> bool {
        false
    }
    fn dead_code_defaults(&self) -> DeadCodeDefaults;
    /// Module keys a project manifest names as entry points.
    fn entry_modules(&self, _project_root: &Path) -> Vec<String> {
        Vec::new()
    }
    /// Files a package manifest names as entry points.
    fn manifest_entry_files(&self, _files: &[EntryCandidate]) -> Vec<PathBuf> {
        Vec::new()
    }
}

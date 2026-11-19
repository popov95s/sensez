use crate::spine::ir::ImportContext;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct FileFacts {
    pub imports: Vec<ImportContext>,
    pub patterns: Vec<Vec<String>>,
    pub unresolved: usize,
}

#[derive(Default)]
pub struct DynamicFacts {
    pub by_file: HashMap<PathBuf, FileFacts>,
    pub unresolved: usize,
}

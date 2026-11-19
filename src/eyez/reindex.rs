use super::{cache, semantic_cache, Index};
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Rebuild {
    Incremental,
    Full,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Warmup {
    Skip,
    SemanticDuplication,
}

pub(crate) struct ReindexReport {
    pub docs: usize,
    pub semantic_warmed: bool,
}

pub(crate) fn reindex(root: &Path, rebuild: Rebuild, warmup: Warmup) -> Result<ReindexReport> {
    if matches!(rebuild, Rebuild::Full) {
        cache::clear(root)?;
        semantic_cache::clear(root)?;
    }
    let index = Index::open(root)?;
    let semantic_warmed = matches!(warmup, Warmup::SemanticDuplication);
    if semantic_warmed {
        crate::analyze_path(root, None)?;
    }
    Ok(ReindexReport {
        docs: index.len(),
        semantic_warmed,
    })
}

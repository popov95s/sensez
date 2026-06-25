//! Crawl + parse a project and collect its documentation elements. Reuses the
//! same discovery/parse path as the scan pipeline; the docs come from the walk's
//! `Walked.docs` (populated only under this feature).

use crate::config::model::Config;
use crate::eyez::DocKind;
use crate::spine::parser::ParsedFile;
use crate::spine::{crawler, parser};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// One documentation element to index.
#[derive(Clone)]
pub struct Doc {
    /// Source file the doc lives in.
    pub file: PathBuf,
    /// 1-indexed line of the doc element.
    pub line: usize,
    pub symbol_path: String,
    pub kind: DocKind,
    pub text: String,
}

/// Discover, parse, and flatten every file's docs under `root`.
pub fn collect(root: &Path) -> Result<Vec<Doc>> {
    let config = Config::load(root).context("loading sensez.toml")?;
    let files = crawler::discover(root, &config.exclude, &|p| {
        crate::profiles::registry::parse_for_path(p).is_some()
    })
    .with_context(|| format!("crawling {}", root.display()))?
    .files;
    let parsed = parser::parse_files(&files);
    Ok(parsed.files.iter().flat_map(docs_of).collect())
}

fn docs_of(file: &ParsedFile) -> Vec<Doc> {
    file.walked
        .docs
        .iter()
        .map(|d| Doc {
            file: file.path.clone(),
            line: d.line,
            symbol_path: d.symbol_path.clone(),
            kind: d.kind,
            text: d.text.clone(),
        })
        .collect()
}

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

/// Comment text attached to a symbol, with file/module context folded in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct CommentBundle {
    pub file: PathBuf,
    pub symbol_path: String,
    pub text: String,
}

/// Build per-symbol comment bundles. Function/class docs are prefixed with
/// top-of-file module comments/docstrings from the same file, because those
/// often carry generated-by/feature intent that local comments omit.
#[allow(dead_code)]
pub fn bundles_with_file_context(docs: &[Doc]) -> Vec<CommentBundle> {
    let mut by_file_module: std::collections::BTreeMap<(PathBuf, String), Vec<&Doc>> =
        std::collections::BTreeMap::new();
    let mut by_symbol: std::collections::BTreeMap<(PathBuf, String), Vec<&Doc>> =
        std::collections::BTreeMap::new();
    for doc in docs.iter().filter(|doc| is_file_context(doc)) {
        by_file_module
            .entry((doc.file.clone(), module_name(&doc.symbol_path).to_string()))
            .or_default()
            .push(doc);
    }
    for doc in docs.iter().filter(|doc| doc.symbol_path.contains("::")) {
        by_symbol
            .entry((doc.file.clone(), doc.symbol_path.clone()))
            .or_default()
            .push(doc);
    }

    by_symbol
        .into_iter()
        .map(|((file, symbol_path), symbol_docs)| {
            let mut parts: Vec<&str> = by_file_module
                .get(&(file.clone(), module_name(&symbol_path).to_string()))
                .into_iter()
                .flat_map(|docs| docs.iter().map(|d| d.text.as_str()))
                .collect();
            parts.extend(symbol_docs.iter().map(|d| d.text.as_str()));
            CommentBundle {
                file,
                symbol_path,
                text: parts.join("\n\n"),
            }
        })
        .collect()
}

/// Discover, parse, and flatten every file's docs under `root`.
pub fn collect(root: &Path) -> Result<Vec<Doc>> {
    let config = Config::load(root).context("loading sensez.toml")?;
    let files = crawler::discover(root, &config.exclude, &|p| {
        crate::profiles::registry::should_parse_path(p)
    })
    .with_context(|| format!("crawling {}", root.display()))?
    .files;
    let parsed = parser::parse_files(&files);
    Ok(parsed.files.iter().flat_map(docs_of).collect())
}

#[allow(dead_code)]
fn is_file_context(doc: &Doc) -> bool {
    !doc.symbol_path.contains("::") && doc.line <= 40
}

#[allow(dead_code)]
fn module_name(symbol_path: &str) -> &str {
    symbol_path.split("::").next().unwrap_or(symbol_path)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(file: &str, line: usize, symbol: &str, text: &str) -> Doc {
        Doc {
            file: PathBuf::from(file),
            line,
            symbol_path: symbol.to_string(),
            kind: DocKind::Comment,
            text: text.to_string(),
        }
    }

    #[test]
    fn bundles_symbol_docs_with_top_file_context() {
        let docs = vec![
            doc("m.py", 1, "m", "file-level purpose"),
            doc("m.py", 80, "m", "late module note"),
            doc("m.py", 12, "m::f", "function purpose"),
            doc("m.py", 13, "m::f", "implementation note"),
        ];

        let bundles = bundles_with_file_context(&docs);
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].symbol_path, "m::f");
        assert!(bundles[0].text.contains("file-level purpose"));
        assert!(bundles[0].text.contains("function purpose"));
        assert!(bundles[0].text.contains("implementation note"));
        assert!(
            !bundles[0].text.contains("late module note"),
            "only top-of-file module docs become context"
        );
    }
}

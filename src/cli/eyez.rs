use super::output;
use crate::eyez::{Rebuild, Warmup};
use crate::reporter::Format;
use anyhow::Result;
use std::path::Path;

pub(super) fn run_reindex(path: &Path, rebuild: Rebuild, warmup: Warmup) -> Result<()> {
    let report = crate::eyez::reindex(path, rebuild, warmup)?;
    output::print_line(&format!(
        "Indexed {} doc/comment item(s).{}",
        report.docs,
        if report.semantic_warmed {
            " Semantic duplication cache warmed."
        } else {
            ""
        }
    ))?;
    Ok(())
}

/// Build/refresh the eyez index for `path` and print the top hits for `query`.
pub(super) fn run_search(
    path: &Path,
    query: &str,
    top_k: std::num::NonZeroUsize,
    format: Format,
) -> Result<()> {
    let index = crate::eyez::Index::open(path)?;
    let hits = index.search(query, top_k);
    if matches!(format, Format::Json) {
        output::print_line(&serde_json::to_string_pretty(&hits)?)?;
    } else if hits.is_empty() {
        output::print_line(&format!(
            "No indexed documentation matched (index size: {}).",
            index.len()
        ))?;
    } else {
        for hit in &hits {
            output::print_line(&format!(
                "{:.3}  {}:{}  {}  [{:?}]\n        {}",
                hit.score, hit.file, hit.line, hit.symbol_path, hit.kind, hit.text
            ))?;
        }
    }
    Ok(())
}

use super::depth::{tree_depth, MAX_TREE_DEPTH};
use super::timing;
use crate::profiles::ParseProfile;
use crate::spine::ir::{FileId, Walked};
use anyhow::{anyhow, Context, Result};

#[cfg(test)]
pub fn parse_source(
    src: &[u8],
    file_id: FileId,
    module_name: &str,
    profile: &dyn ParseProfile,
) -> Result<Walked> {
    let mut parser = tree_sitter::Parser::new();
    parse_source_with_parser(src, file_id, module_name, profile, &mut parser)
}

pub(super) fn parse_source_with_parser(
    src: &[u8],
    file_id: FileId,
    module_name: &str,
    profile: &dyn ParseProfile,
    parser: &mut tree_sitter::Parser,
) -> Result<Walked> {
    parser
        .set_language(&profile.ts_language())
        .context("incompatible tree-sitter grammar ABI")?;
    let tree_started = timing::enabled().then(std::time::Instant::now);
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| anyhow!("tree-sitter returned no tree"))?;
    if let Some(started) = tree_started {
        timing::record_tree(started.elapsed());
    }
    let depth_started = timing::enabled().then(std::time::Instant::now);
    if tree_depth(tree.root_node(), MAX_TREE_DEPTH) > MAX_TREE_DEPTH {
        return Err(anyhow!(
            "syntax tree deeper than {MAX_TREE_DEPTH} levels; skipping (DoS guard)"
        ));
    }
    if let Some(started) = depth_started {
        timing::record_depth(started.elapsed());
    }
    let walk_started = timing::enabled().then(std::time::Instant::now);
    let mut walked = profile.walk(tree.root_node(), src, file_id, module_name);
    walked.shrink_to_fit();
    if let Some(started) = walk_started {
        timing::record_walk(started.elapsed());
    }
    Ok(walked)
}

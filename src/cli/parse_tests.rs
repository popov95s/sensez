//! Argument parsing.

use super::*;
use clap::Parser;

#[test]
fn all_conflicts_with_max() {
    assert!(spec::Cli::try_parse_from(["sensez", "noze", ".", "--all", "--max", "1"]).is_err());
}

#[cfg(feature = "eyez")]
#[test]
fn eyez_top_k_zero_rejected_by_parser() {
    let args = ["sensez", "eyez", "search", ".", "query", "--top-k", "0"];
    assert!(spec::Cli::try_parse_from(args).is_err());
}

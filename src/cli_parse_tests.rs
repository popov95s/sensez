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

#[cfg(feature = "eyez")]
#[test]
fn eyez_search_takes_optional_path_before_query() {
    for (argv, expected) in [
        (
            vec!["sensez", "eyez", "search", "parse date"],
            vec!["parse date"],
        ),
        (vec!["sensez", "eyez", "search", ".", "q"], vec![".", "q"]),
    ] {
        let cli = spec::Cli::try_parse_from(argv).unwrap();
        let Some(spec::Command::Eyez(eyez)) = cli.command else {
            panic!("expected eyez");
        };
        let spec::EyezAction::Search { args, .. } = eyez.action else {
            panic!("expected search");
        };
        assert_eq!(args, expected);
    }
    assert!(spec::Cli::try_parse_from(["sensez", "eyez", "reindex"]).is_ok());
}

#[test]
fn fail_on_dot_error_mentions_path_order() {
    let error = spec::Cli::try_parse_from(["sensez", "noze", "--fail-on", "."])
        .unwrap_err()
        .to_string();
    assert!(error.contains("put it before the flag"), "{error}");
}

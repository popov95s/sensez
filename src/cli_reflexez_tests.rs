use super::spec::{Cli, Command};
use clap::Parser;
use std::path::PathBuf;

#[test]
fn preserves_explicit_files_and_runner_arguments() {
    let cli = Cli::try_parse_from([
        "sensez",
        "reflexez",
        ".",
        "--changed-file",
        "src/a.ts",
        "--",
        "--reporter=dot",
    ])
    .unwrap();
    let Some(Command::Reflexez(args)) = cli.command else {
        panic!("expected reflexez command");
    };
    assert_eq!(args.changed_files, vec![PathBuf::from("src/a.ts")]);
    assert_eq!(args.runner_args, vec!["--reporter=dot"]);
}

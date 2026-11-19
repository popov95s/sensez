use super::{render, runners, selector};
use crate::cli::spec::ReflexezArgs;
use anyhow::Result;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run(args: ReflexezArgs) -> Result<ExitCode> {
    let root = args.path.as_deref().unwrap_or_else(|| Path::new("."));
    let root = std::fs::canonicalize(root)?;
    let plan = selector::plan(&root, &args)?;
    if args.json {
        crate::cli::output::print_line(&serde_json::to_string_pretty(&plan)?)?;
        return Ok(ExitCode::SUCCESS);
    }
    crate::cli::output::print_line(&render::terminal(&plan))?;
    if args.plan {
        return Ok(ExitCode::SUCCESS);
    }
    runners::execute(&plan, &args.runner_args)
}

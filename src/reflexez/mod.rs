//! Changed-code test impact analysis that delegates execution to existing runners.

mod changes;
mod discovery;
mod dynamic;
mod model;
mod patterns;
mod render;
mod runners;
mod selector;
mod selector_dynamic;

use crate::cli::spec::ReflexezArgs;
use anyhow::Result;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run(args: ReflexezArgs) -> Result<ExitCode> {
    let root = args.path.as_deref().unwrap_or_else(|| Path::new("."));
    let root = std::fs::canonicalize(root)?;
    let plan = selector::plan(&root, &args)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
        return Ok(ExitCode::SUCCESS);
    }
    println!("{}", render::terminal(&plan, &root));
    if args.plan {
        return Ok(ExitCode::SUCCESS);
    }
    runners::execute(&plan, &args.runner_args)
}

#[cfg(test)]
mod tests;

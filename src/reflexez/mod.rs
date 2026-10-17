//! Changed-code test impact analysis that delegates execution to existing runners.

mod assets;
mod changes;
mod discovery;
mod dynamic;
mod model;
mod module_root;
mod patterns;
mod reach;
mod render;
mod runner_config;
mod runners;
mod selector;
mod selector_dynamic;
mod setup_trigger;

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
    crate::cli::output::print_line(&render::terminal(&plan, &root))?;
    if args.plan {
        return Ok(ExitCode::SUCCESS);
    }
    runners::execute(&plan, &args.runner_args)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_config;
#[cfg(test)]
mod tests_declaration;
#[cfg(test)]
mod tests_graph;
#[cfg(test)]
mod tests_settings;

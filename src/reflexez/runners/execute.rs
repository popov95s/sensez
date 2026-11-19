use crate::reflexez::model::{ImpactPlan, RunMode, RunnerKind, RunnerPlan};
use anyhow::{Context, Result};
use std::process::{Command, ExitCode};

pub fn execute(plan: &ImpactPlan, runner_args: &[String]) -> Result<ExitCode> {
    if plan.runners.is_empty() {
        println!("sensez reflexez: no affected tests");
        return Ok(ExitCode::SUCCESS);
    }
    let mut failed = false;
    for runner in &plan.runners {
        let status = Command::new(&runner.program)
            .args(&runner.prefix_args)
            .args(&runner.tests)
            .args(runner_args)
            .current_dir(&runner.root)
            .status()
            .with_context(|| {
                format!(
                    "starting {} at {} (install it in the project or select another runner)",
                    runner.kind.label(),
                    runner.program.display()
                )
            })?;
        failed |= !status.success() && !is_empty_narrowed_pytest(runner, status.code());
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// pytest exits 5 when it collects nothing, which a narrowed selection may legitimately do.
pub(super) fn is_empty_narrowed_pytest(runner: &RunnerPlan, code: Option<i32>) -> bool {
    runner.kind == RunnerKind::Pytest && runner.mode == RunMode::Narrowed && code == Some(5)
}

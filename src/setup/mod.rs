mod artifacts;
mod prompts;

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub struct InitOptions {
    pub path: Option<PathBuf>,
    pub agent: Option<String>,
    pub gate: bool,
    pub no_metrics: bool,
    pub yes: bool,
}

pub fn run(opts: InitOptions) -> Result<()> {
    let root = resolve_root(opts.path.as_deref())?;
    let interactive = prompts::interactive();
    if !interactive && opts.agent.is_none() && !opts.yes {
        bail!(
            "no TTY and no flags — run interactively, or pass e.g. \
             `sense init --agent claude-code --gate --yes` (see --help)"
        );
    }

    let agent = match opts.agent.as_deref() {
        Some(a @ ("claude-code" | "cursor" | "other")) => a.to_string(),
        Some(other) => bail!("unknown --agent '{other}' (claude-code | cursor | other)"),
        None if interactive => {
            let choice = prompts::select(
                "Which coding agent will use Sensez here?",
                &["Claude Code", "Cursor", "Other / none (print instructions)"],
                0,
            )?;
            ["claude-code", "cursor", "other"][choice].to_string()
        }
        None => "claude-code".to_string(),
    };

    let gate = if agent == "claude-code" {
        opts.gate
            || (interactive
                && !opts.yes
                && prompts::confirm(
                    "Also check the agent's changes at the end of every turn?\n\
                     (installs a Stop hook running a diff-scoped scan; blocks at \
                     most once, so false positives never trap the agent)",
                    true,
                )?)
    } else {
        if opts.gate {
            eprintln!("note: --gate is Claude Code-only (Stop hooks); skipping.");
        }
        false
    };

    let metrics_enabled = if opts.no_metrics {
        false
    } else if interactive && !opts.yes {
        prompts::confirm(
            "Enable sensez self-improvement?\n\
             sensez learns from this session — which findings you fix, which you \
             dismiss as false positives — to tune what it surfaces and to show \
             its value (`brainz_report`). That session data is saved ONLY on your \
             disk (.sensez/local-metrics/) and is NEVER transmitted anywhere. \
             Declining stops the on-disk recording; sensez still works fully.",
            true,
        )?
    } else {
        true
    };

    let has_pyproject = root.join("pyproject.toml").exists();
    let into_pyproject = has_pyproject
        && !root.join("sensez.toml").exists()
        && interactive
        && !opts.yes
        && prompts::confirm(
            "pyproject.toml found — keep Sensez' config there ([tool.sensez]) \
             instead of creating sensez.toml?",
            false,
        )?;

    let sensez_bin = std::env::current_exe()
        .context("locating the sensez executable")?
        .to_string_lossy()
        .into_owned();
    let mut done = vec![artifacts::write_config(
        &root,
        metrics_enabled,
        into_pyproject,
    )?];
    if agent == "other" {
        done.push(
            "any MCP client works: speak JSON-RPC over stdio to `sense mcp serve` \
             (tools: noze_sniff, eyez_search_docs, brainz_triage, brainz_report)"
                .to_string(),
        );
    } else {
        done.push(artifacts::write_mcp_config(&root, &agent, &sensez_bin)?);
    }
    if gate {
        done.push(artifacts::write_gate(&root)?);
    }
    done.push(artifacts::ensure_gitignore(&root)?);

    println!("\nSensez is set up in {}:", root.display());
    for line in &done {
        println!("  • {line}");
    }
    println!(
        "\nYou're running on defaults. Everything sensez does — thresholds, \
         excludes, boundaries, self-improvement — is configured in {}.\n{}",
        if into_pyproject {
            "pyproject.toml [tool.sensez]"
        } else {
            "sensez.toml"
        },
        if agent == "claude-code" {
            "Reload your Claude Code window to pick up the MCP server."
        } else {
            "Restart your agent to pick up the MCP server."
        }
    );
    Ok(())
}

fn resolve_root(path: Option<&Path>) -> Result<PathBuf> {
    let root = match path {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().context("getting current directory")?,
    };
    let root = root
        .canonicalize()
        .with_context(|| format!("resolving {}", root.display()))?;
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }
    if !root.join(".git").exists() {
        if let Some(repo) = root.ancestors().skip(1).find(|a| a.join(".git").exists()) {
            eprintln!(
                "note: {} is a subdirectory of the repository at {} — Sensez' \
                 graph analysis is only correct over the full repo; consider \
                 running `sense init {}` instead.",
                root.display(),
                repo.display(),
                repo.display()
            );
        }
    }
    Ok(root)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

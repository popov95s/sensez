use super::agents::AgentSpec;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallScope {
    Project,
    Global,
}

pub struct Target {
    pub requested_root: PathBuf,
    pub repository_root: Option<PathBuf>,
}

impl Target {
    pub fn resolve(path: Option<&Path>) -> Result<Self> {
        let requested = match path {
            Some(path) => path.to_path_buf(),
            None => std::env::current_dir().context("getting current directory")?,
        };
        let requested_root = requested
            .canonicalize()
            .with_context(|| format!("resolving {}", requested.display()))?;
        if !requested_root.is_dir() {
            bail!("{} is not a directory", requested_root.display());
        }
        let repository_root = requested_root
            .ancestors()
            .find(|candidate| candidate.join(".git").exists())
            .map(Path::to_path_buf);
        Ok(Self {
            requested_root,
            repository_root,
        })
    }

    pub fn warn_if_nested(&self) {
        let Some(repo) = self
            .repository_root
            .as_ref()
            .filter(|repo| *repo != &self.requested_root)
        else {
            return;
        };
        eprintln!(
            "note: {} is a subdirectory of the repository at {} — Sensez' graph analysis is only correct over the full repo; consider running `sensez init {}` instead.",
            self.requested_root.display(),
            repo.display(),
            repo.display()
        );
    }
}

pub fn user_home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            anyhow::anyhow!("cannot install globally: user home directory is unavailable")
        })
}

pub fn destination_summary(home: &Path, agent: &AgentSpec) -> String {
    let destinations: Vec<String> = [agent.global_mcp_relpath, agent.global_skill_relpath]
        .into_iter()
        .flatten()
        .map(|relative| home.join(relative).display().to_string())
        .collect();
    if destinations.is_empty() {
        "your agent's user-level MCP settings".to_string()
    } else {
        destinations.join(" and ")
    }
}

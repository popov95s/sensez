//! Locates the embedding model on disk, downloading a pinned revision on first use.

use super::embed::MODEL_ID;
use anyhow::{Context, Result};
use hf_hub::api::sync::{Api, ApiBuilder};
use hf_hub::{Cache, Repo, RepoType};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

const MODEL_REVISION: &str = "6fc8051fab2a1e0ee76689cf08c853792ac285e7";
const MODEL_FILES: [&str; 3] = ["config.json", "tokenizer.json", "model.safetensors"];

/// Where the model comes from, decided without touching the network.
#[derive(Debug, PartialEq, Eq)]
enum Source {
    Local(PathBuf),
    Download,
}

/// Model-related environment: `SENSEZ_EYEZ_MODEL_DIR`, `HF_HOME` and the offline switches.
struct ModelEnv {
    model_dir: Option<PathBuf>,
    cache: Cache,
    offline: bool,
}

impl ModelEnv {
    fn from_env() -> Self {
        Self {
            model_dir: std::env::var_os("SENSEZ_EYEZ_MODEL_DIR").map(PathBuf::from),
            cache: Cache::from_env(),
            offline: ["HF_HUB_OFFLINE", "SENSEZ_OFFLINE"]
                .iter()
                .any(|key| std::env::var(key).is_ok_and(|value| is_truthy(&value))),
        }
    }
}

/// Directory holding the model files, downloading them first when needed.
pub(crate) fn model_dir() -> Result<PathBuf> {
    let env = ModelEnv::from_env();
    match resolve(&env)? {
        Source::Local(dir) => Ok(dir),
        Source::Download => download(&env.cache),
    }
}

fn resolve(env: &ModelEnv) -> Result<Source> {
    if let Some(dir) = &env.model_dir {
        return Ok(Source::Local(dir.clone()));
    }
    if let Some(snapshot) = cached_snapshot(&env.cache) {
        return Ok(Source::Local(snapshot));
    }
    if env.offline {
        anyhow::bail!(
            "eyez model {MODEL_ID} is not cached under {} and offline mode is set \
             (HF_HUB_OFFLINE / SENSEZ_OFFLINE); set SENSEZ_EYEZ_MODEL_DIR to a local copy",
            env.cache.path().display()
        );
    }
    Ok(Source::Download)
}

/// Snapshot directory of the pinned revision when every model file is cached.
fn cached_snapshot(cache: &Cache) -> Option<PathBuf> {
    let repo = cache.repo(model_repo());
    let files: Option<Vec<PathBuf>> = MODEL_FILES.iter().map(|file| repo.get(file)).collect();
    match files {
        Some(files) => files.first()?.parent().map(Path::to_path_buf),
        None => sha_snapshot(cache),
    }
}

/// The pinned sha's snapshot when every file is there without a ref, as Python `huggingface_hub` leaves it.
fn sha_snapshot(cache: &Cache) -> Option<PathBuf> {
    let snapshot = cache
        .path()
        .join(model_repo().folder_name())
        .join("snapshots")
        .join(MODEL_REVISION);
    let complete = MODEL_FILES.iter().all(|file| snapshot.join(file).is_file());
    (is_commit_sha(MODEL_REVISION) && complete).then_some(snapshot)
}

fn is_commit_sha(revision: &str) -> bool {
    revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit())
}

fn download(cache: &Cache) -> Result<PathBuf> {
    eprintln!("{}", download_notice(cache.path()));
    let repo = hub_client(cache)?.repo(model_repo());
    let mut snapshot = None;
    for file in MODEL_FILES {
        let path = repo
            .get(file)
            .with_context(|| format!("downloading {file} of {MODEL_ID}"))?;
        snapshot = path.parent().map(Path::to_path_buf);
    }
    snapshot.context("the model download returned no file")
}

/// Hub client writing into `cache`.
fn hub_client(cache: &Cache) -> Result<Api> {
    ApiBuilder::from_cache(cache.clone())
        .with_progress(std::io::stderr().is_terminal())
        .build()
        .context("starting the HuggingFace Hub client")
}

fn model_repo() -> Repo {
    Repo::with_revision(
        MODEL_ID.to_string(),
        RepoType::Model,
        MODEL_REVISION.to_string(),
    )
}

fn download_notice(cache_dir: &Path) -> String {
    format!(
        "[sensez] eyez: first use downloads {MODEL_ID} (~130 MB) to {} \
         (set HF_HOME or SENSEZ_EYEZ_MODEL_DIR to change)",
        cache_dir.display()
    )
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;

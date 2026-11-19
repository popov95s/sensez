//! Shared infrastructure for integration-style unit tests.

mod files;
mod git_repo;
mod metrics_guard;

pub(crate) use files::write;
pub(crate) use git_repo::GitTestRepo;
pub(crate) use metrics_guard::metrics_guard;

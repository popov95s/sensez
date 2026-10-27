//! One workspace scan on a background thread, reported back as a [`ScanResult`].

use super::diagnostics::{from_report, retain_visible, Diagnostic};
use super::health::HealthSummary;
use super::settings::{AnalysisScope, Settings};
use crossbeam_channel::Sender;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;

static SCAN_THREADS: OnceLock<Mutex<Vec<thread::JoinHandle<()>>>> = OnceLock::new();

pub(super) struct ScanResult {
    pub root: PathBuf,
    pub generation: u64,
    pub diagnostics: BTreeMap<String, Vec<Diagnostic>>,
    pub health: Option<HealthSummary>,
    pub error: Option<String>,
    pub notice: Option<String>,
}

impl ScanResult {
    fn failed(
        root: PathBuf,
        generation: u64,
        health: Option<HealthSummary>,
        error: &anyhow::Error,
    ) -> Self {
        Self {
            root,
            generation,
            diagnostics: BTreeMap::new(),
            health,
            error: Some(format!("{error:#}")),
            notice: None,
        }
    }
}

pub(super) fn spawn(
    root: PathBuf,
    generation: u64,
    settings: Settings,
    completed: Sender<ScanResult>,
) {
    let handle = thread::spawn(move || {
        let _ = completed.send(scan(root, generation, settings));
    });
    SCAN_THREADS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(handle);
}

pub(super) fn join_all() {
    if let Some(handles) = SCAN_THREADS.get() {
        let mut guards = handles
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for handle in guards.drain(..) {
            let _ = handle.join();
        }
    }
}

/// Scan `root`; outside a git work tree the changed scope falls back to the workspace.
pub(super) fn scan(root: PathBuf, generation: u64, settings: Settings) -> ScanResult {
    let (mut report, module_files) = match crate::analyze_path_in_service(&root, None) {
        Ok(scanned) => scanned,
        Err(error) => return ScanResult::failed(root, generation, None, &error),
    };
    let mut health = settings
        .health_enabled
        .then(|| HealthSummary::from_report(root.display().to_string(), "workspace", &report));
    let mut notice = None;
    if settings.scope == AnalysisScope::Changed {
        if outside_git(&root) {
            notice = Some(format!(
                "Sensez: {} is not in a git work tree; showing the whole workspace",
                root.display()
            ));
        } else {
            match crate::diff::git::changed_vs_head(&root) {
                Ok(changed) => crate::diff::apply(&mut report, &changed, &module_files),
                Err(error) => return ScanResult::failed(root, generation, health, &error),
            }
        }
    }
    if let Some(health) = health.as_mut() {
        health.set_current_changes(&report);
    }
    retain_visible(&mut report, settings);
    ScanResult {
        root,
        generation,
        diagnostics: from_report(&report),
        health,
        error: None,
        notice,
    }
}

fn outside_git(root: &Path) -> bool {
    crate::diff::git::repository_root(root).is_err()
}

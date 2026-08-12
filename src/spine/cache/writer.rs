use super::{AnalysisSnapshot, SnapshotCache};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::JoinHandle;

const MAX_PENDING_REPOSITORIES: usize = 8;
static ENABLED: AtomicBool = AtomicBool::new(false);
static WRITER: OnceLock<BackgroundWriter> = OnceLock::new();

struct WriteJob {
    cache: SnapshotCache,
    key: u64,
    snapshot: AnalysisSnapshot,
}

#[derive(Default)]
struct Queue {
    pending: HashMap<PathBuf, WriteJob>,
    active: bool,
    shutdown: bool,
}

struct Shared {
    queue: Mutex<Queue>,
    changed: Condvar,
}

struct BackgroundWriter {
    shared: Arc<Shared>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl BackgroundWriter {
    fn start() -> Option<Self> {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            changed: Condvar::new(),
        });
        let worker_shared = Arc::clone(&shared);
        let handle = std::thread::Builder::new()
            .name("sensez-cache-writer".into())
            .spawn(move || run(worker_shared))
            .ok()?;
        Some(Self {
            shared,
            handle: Mutex::new(Some(handle)),
        })
    }

    fn disabled() -> Self {
        Self {
            shared: Arc::new(Shared {
                queue: Mutex::new(Queue {
                    shutdown: true,
                    ..Queue::default()
                }),
                changed: Condvar::new(),
            }),
            handle: Mutex::new(None),
        }
    }

    fn is_active(&self) -> bool {
        self.handle.lock().is_ok_and(|handle| handle.is_some())
    }

    fn flush(&self) {
        let Ok(queue) = self.shared.queue.lock() else {
            return;
        };
        let _guard = self
            .shared
            .changed
            .wait_while(queue, |state| state.active || !state.pending.is_empty());
    }

    fn shutdown(&self) {
        self.flush();
        if let Ok(mut queue) = self.shared.queue.lock() {
            queue.shutdown = true;
            self.shared.changed.notify_all();
        }
        if let Ok(mut handle) = self.handle.lock() {
            if let Some(worker) = handle.take() {
                let _ = worker.join();
            }
        }
    }
}

pub fn enable_background_writes() {
    ENABLED.store(true, Ordering::Release);
}

pub fn flush_background_writes() {
    let Some(writer) = WRITER.get() else {
        return;
    };
    writer.flush();
}

pub fn shutdown_background_writes() {
    ENABLED.store(false, Ordering::Release);
    let Some(writer) = WRITER.get() else {
        return;
    };
    writer.shutdown();
}

pub(crate) fn persist(cache: SnapshotCache, key: u64, snapshot: AnalysisSnapshot) {
    let mut job = WriteJob {
        cache,
        key,
        snapshot,
    };
    if ENABLED.load(Ordering::Acquire) {
        let writer = WRITER
            .get_or_init(|| BackgroundWriter::start().unwrap_or_else(BackgroundWriter::disabled));
        if writer.is_active() {
            match submit(writer, job) {
                Ok(()) => return,
                Err(returned) => job = *returned,
            }
        }
    }
    let _ = job.cache.persist(job.key, &job.snapshot);
}

fn submit(writer: &BackgroundWriter, job: WriteJob) -> Result<(), Box<WriteJob>> {
    let path = job.cache.path_key();
    let Ok(mut queue) = writer.shared.queue.lock() else {
        return Err(Box::new(job));
    };
    if queue.shutdown
        || (!queue.pending.contains_key(&path) && queue.pending.len() >= MAX_PENDING_REPOSITORIES)
    {
        return Err(Box::new(job));
    }
    queue.pending.insert(path, job);
    writer.shared.changed.notify_one();
    Ok(())
}

fn run(shared: Arc<Shared>) {
    loop {
        let job = {
            let Ok(queue) = shared.queue.lock() else {
                return;
            };
            let Ok(mut queue) = shared
                .changed
                .wait_while(queue, |state| state.pending.is_empty() && !state.shutdown)
            else {
                return;
            };
            if queue.shutdown && queue.pending.is_empty() {
                return;
            }
            let Some(path) = queue.pending.keys().next().cloned() else {
                continue;
            };
            queue.active = true;
            queue.pending.remove(&path)
        };

        if let Some(job) = job {
            if let Ok(Some(prepared)) = SnapshotCache::prepare(job.key, &job.snapshot) {
                let _ = job.cache.write(prepared);
            }
        }
        if let Ok(mut queue) = shared.queue.lock() {
            queue.active = false;
            shared.changed.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::AnalysisReport;

    #[test]
    fn flush_persists_latest_snapshot_for_repository() {
        let root = tempfile::tempdir().unwrap();
        let cache = SnapshotCache::new(root.path());
        let writer = BackgroundWriter::start().unwrap();
        for key in 1..=20 {
            let mut report = AnalysisReport::default();
            report.meta.analyzed_files = key;
            let snapshot = AnalysisSnapshot::new(report, HashMap::new());
            submit(
                &writer,
                WriteJob {
                    cache: cache.clone(),
                    key: key as u64,
                    snapshot,
                },
            )
            .map_err(|_| ())
            .unwrap();
        }

        writer.flush();
        assert_eq!(cache.load(20).unwrap().report.meta.analyzed_files, 20);
        writer.shutdown();
    }
}

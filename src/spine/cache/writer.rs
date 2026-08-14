use super::{ParseCache, ParseWriteInput};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::JoinHandle;

const MAX_PENDING_REPOSITORIES: usize = 8;
static ENABLED: AtomicBool = AtomicBool::new(false);
static WRITER: OnceLock<BackgroundWriter> = OnceLock::new();

struct WriteJob {
    parse_cache: ParseCache,
    parse: Option<ParseWriteInput>,
    remove_parse: bool,
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

pub(crate) fn persist(parse_cache: ParseCache, parse: Option<ParseWriteInput>, remove_parse: bool) {
    let mut job = WriteJob {
        parse_cache,
        parse,
        remove_parse,
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
    persist_job(job);
}

fn submit(writer: &BackgroundWriter, job: WriteJob) -> Result<(), Box<WriteJob>> {
    let path = job.parse_cache.path_key();
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
            persist_job(job);
        }
        if let Ok(mut queue) = shared.queue.lock() {
            queue.active = false;
            shared.changed.notify_all();
        }
    }
}

fn persist_job(job: WriteJob) {
    let refresh_parse = job.parse.is_some();
    let parsed = job.parse.and_then(|input| {
        ParseCache::prepare(input, super::budget::TOTAL_BYTES)
            .ok()
            .flatten()
    });
    match parsed {
        Some(prepared) => {
            let _ = job.parse_cache.write(prepared);
        }
        None if job.remove_parse
            || refresh_parse
            || job.parse_cache.len() > super::budget::TOTAL_BYTES =>
        {
            job.parse_cache.remove();
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::cache::SourceFile;

    #[test]
    fn flush_persists_latest_parse_cache_for_repository() {
        let root = tempfile::tempdir().unwrap();
        let parse_cache = ParseCache::new(root.path());
        let writer = BackgroundWriter::start().unwrap();
        for _ in 1..=20 {
            let parse = Some(ParseCache::capture(&[] as &[SourceFile], Vec::new()));
            submit(
                &writer,
                WriteJob {
                    parse_cache: parse_cache.clone(),
                    parse,
                    remove_parse: false,
                },
            )
            .map_err(|_| ())
            .unwrap();
        }

        writer.flush();
        assert!(parse_cache.len() <= super::super::budget::TOTAL_BYTES);
        writer.shutdown();
    }
}

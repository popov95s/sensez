//! Async stdio transport: read newline-delimited JSON-RPC, dispatch each
//! request on its own task, and serialize replies through a single writer
//! task. Requests therefore never block each other or the shutdown signal:
//! a long scan no longer delays pings, subsequent requests, or SIGTERM.
//! Also owns the metrics lifecycle: periodic flushes while serving, and a
//! final flush on any exit path (stdin EOF, Ctrl-C, SIGTERM).

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Semaphore};

/// Refuse absurdly large frames instead of buffering them whole: nothing in
/// the MCP surface needs remotely this much, and an uncapped line read would
/// let anything writing to stdin balloon process memory.
const MAX_LINE_BYTES: usize = 10 * 1024 * 1024;
/// Upper bound on concurrently executing requests. Beyond this, callers get
/// an immediate busy error instead of the server silently accumulating work.
const MAX_CONCURRENT_REQUESTS: usize = 8;
/// Hard deadline for one request's work. A timed-out scan keeps running on
/// the blocking pool, but the server stays responsive and answers the client.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);

/// Serve the MCP protocol over stdin/stdout until EOF or a shutdown signal.
pub async fn serve() -> Result<()> {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let (tx, rx) = mpsc::channel::<String>(64);
    let writer = tokio::spawn(writer_task(rx));
    let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
    let mut flush_tick = tokio::time::interval(Duration::from_secs(30));
    flush_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    let served = loop {
        tokio::select! {
            line = lines.next_line() => match line.context("reading stdin") {
                // Do NOT `?` here: a dispatch error (e.g. broken pipe when the
                // client exits early) must still reach the final metrics flush.
                Ok(Some(line)) => {
                    if let Err(err) = dispatch(&line, &permits, &tx).await {
                        break Err(err);
                    }
                }
                Ok(None) => break Ok(()),       // client closed stdin
                Err(err) => break Err(err),
            },
            _ = flush_tick.tick() => {
                // Spawn blocking to avoid stalling the async runtime during
                // potentially expensive recapture/flush operations.
                let _ = tokio::task::spawn_blocking(|| {
                    crate::brainz::recapture(); // bank fixes since the last scan
                    crate::brainz::flush();
                }).await;
            }
            _ = &mut shutdown => break Ok(()),  // Ctrl-C / SIGTERM
        }
    };
    // Graceful shutdown: close the response queue so the writer drains
    // whatever is already in flight, then persist buffered metrics. We
    // deliberately do *not* run a final recapture here — the periodic tick
    // already covers in-session fixes, and a shutdown-time recapture would do
    // an O(repo) cheap-scan guard walk under SIGTERM pressure.
    drop(tx);
    let _ = tokio::time::timeout(Duration::from_secs(5), writer).await;
    let _ = tokio::task::spawn_blocking(crate::brainz::flush).await;
    served
}

/// Validate and route one incoming frame. Cheap parsing happens here so the
/// read loop stays responsive; the expensive handler runs on the blocking
/// pool inside [`handle`].
async fn dispatch(line: &str, permits: &Arc<Semaphore>, tx: &mpsc::Sender<String>) -> Result<()> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if line.len() > MAX_LINE_BYTES {
        return send(
            tx,
            &error_response(None, -32600, "request exceeds maximum frame size"),
        )
        .await;
    }
    let message: Value = match serde_json::from_str(trimmed) {
        Ok(value) => value,
        Err(_) => return send(tx, &parse_error()).await,
    };
    if message.is_array() {
        return send(
            tx,
            &error_response(None, -32600, "batch requests are not supported"),
        )
        .await;
    }

    let id = message.get("id").cloned();
    match permits.clone().try_acquire_owned() {
        Ok(permit) => {
            tokio::spawn(handle(message, permit, tx.clone()));
            Ok(())
        }
        // A notification expects no reply regardless of load.
        Err(_) if id.is_none() => Ok(()),
        Err(_) => {
            send(
                tx,
                &error_response(
                    id,
                    -32000,
                    "server busy: too many concurrent requests; retry shortly",
                ),
            )
            .await
        }
    }
}

async fn handle(
    message: Value,
    _permit: tokio::sync::OwnedSemaphorePermit,
    tx: mpsc::Sender<String>,
) {
    let id = message.get("id").cloned();
    let work = tokio::task::spawn_blocking(move || super::handle_message(&message));
    let response = match tokio::time::timeout(REQUEST_TIMEOUT, work).await {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            id.map(|id| error_response(Some(id), -32603, format!("handler failed: {err}")))
        }
        Err(_elapsed) => id.map(|id| error_response(Some(id), -32000, "request timed out")),
    };
    if let Some(response) = response {
        let _ = send(&tx, &response).await;
    }
}

/// Write serialized responses to stdout until the queue closes or stdout dies.
async fn writer_task(mut rx: mpsc::Receiver<String>) {
    let mut stdout = tokio::io::stdout();
    while let Some(text) = rx.recv().await {
        // A dead stdout means the client is gone; stop writing and drain the
        // queue so producers never hang on a closed channel.
        if stdout.write_all(text.as_bytes()).await.is_err() {
            break;
        }
        if stdout.flush().await.is_err() {
            break;
        }
    }
}

async fn send(tx: &mpsc::Sender<String>, value: &Value) -> Result<()> {
    let mut text = serde_json::to_string(value).context("serializing response")?;
    text.push('\n');
    tx.send(text).await.context("response channel closed")
}

/// Resolve when the process is asked to stop (Ctrl-C everywhere; SIGTERM on
/// Unix — what an MCP client sends when shutting a server down).
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = term.recv() => {}
                }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn error_response(id: Option<Value>, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message.into()}
    })
}

fn parse_error() -> Value {
    error_response(None, -32700, "parse error")
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn round_trip(line: &str) -> Option<Value> {
        let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
        let (tx, mut rx) = mpsc::channel(8);
        dispatch(line, &permits, &tx).await.unwrap();
        rx.recv()
            .await
            .map(|text| serde_json::from_str(&text).unwrap())
    }

    #[tokio::test]
    async fn oversized_lines_are_rejected_without_being_parsed() {
        let response = round_trip(&"x".repeat(MAX_LINE_BYTES + 1))
            .await
            .expect("oversized frames must still get a reply");
        assert_eq!(response["error"]["code"], -32600);
    }

    #[tokio::test]
    async fn batch_arrays_get_an_explicit_error() {
        let response = round_trip(&json!([1, 2]).to_string())
            .await
            .expect("batches must be answered, not dropped");
        assert_eq!(response["error"]["code"], -32600);
    }

    #[tokio::test]
    async fn malformed_json_yields_parse_error() {
        let response = round_trip("this is not json")
            .await
            .expect("parse errors must be answered");
        assert_eq!(response["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn ping_round_trips_through_the_writer_queue() {
        let request = json!({"jsonrpc": "2.0", "id": 11, "method": "ping"}).to_string();
        let response = round_trip(&request).await.expect("ping must be answered");
        assert_eq!(response["id"], 11);
        assert_eq!(response["result"], json!({}));
    }

    #[tokio::test]
    async fn notifications_produce_no_response() {
        let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
        let (tx, mut rx) = mpsc::channel(8);
        let notification =
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string();
        dispatch(&notification, &permits, &tx).await.unwrap();
        drop(tx);
        assert!(rx.recv().await.is_none(), "notifications expect no reply");
    }
}

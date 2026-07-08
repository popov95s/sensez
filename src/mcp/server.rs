//! Async stdio transport: read newline-delimited JSON-RPC, dispatch, reply.
//! Also owns the metrics lifecycle: periodic flushes while serving, and a
//! final flush on any exit path (stdin EOF, Ctrl-C, SIGTERM).

use anyhow::{Context, Result};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Serve the MCP protocol over stdin/stdout until EOF or a shutdown signal.
pub async fn serve() -> Result<()> {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    let mut flush_tick = tokio::time::interval(Duration::from_secs(30));
    flush_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    let served = loop {
        tokio::select! {
            line = lines.next_line() => match line.context("reading stdin") {
                // Do NOT `?` here: a transport error (e.g. broken pipe when the
                // client exits early) must still reach the final metrics flush.
                Ok(Some(line)) => {
                    if let Err(err) = handle_line(&line, &mut stdout).await {
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
    // Graceful shutdown: just persist buffered metrics. We deliberately do
    // *not* run a final recapture here — the periodic tick already covers
    // in-session fixes, and a shutdown-time recapture would do an O(repo)
    // cheap-scan guard walk under SIGTERM pressure. If the client cleanly
    // disconnects mid-session, the last periodic flush is fresh enough.
    let _ = tokio::task::spawn_blocking(|| {
        crate::brainz::flush();
    })
    .await;
    served
}

async fn handle_line(line: &str, stdout: &mut tokio::io::Stdout) -> Result<()> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let message: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(value) => value,
        Err(_) => return write_line(stdout, &parse_error()).await,
    };
    // Spawn blocking to avoid stalling the async runtime during potentially
    // expensive operations like scans.
    let response = tokio::task::spawn_blocking(move || super::handle_message(&message))
        .await
        .context("spawn_blocking handle_message")?;
    if let Some(response) = response {
        write_line(stdout, &response).await?;
    }
    Ok(())
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

async fn write_line(out: &mut tokio::io::Stdout, value: &serde_json::Value) -> Result<()> {
    let mut text = serde_json::to_string(value).context("serializing response")?;
    text.push('\n');
    out.write_all(text.as_bytes())
        .await
        .context("writing stdout")?;
    out.flush().await.context("flushing stdout")?;
    Ok(())
}

fn parse_error() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": null,
        "error": {"code": -32700, "message": "parse error"}
    })
}

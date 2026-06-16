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
                crate::brainz::recapture(); // bank fixes since the last scan
                crate::brainz::flush();
            }
            _ = &mut shutdown => break Ok(()),  // Ctrl-C / SIGTERM
        }
    };
    // Graceful shutdown: one last automatic fix-recapture pass (bounded by the
    // cheap-scan guard), then persist buffered metrics — even when the
    // transport errored.
    crate::brainz::recapture();
    crate::brainz::flush();
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
    if let Some(response) = super::handle_message(&message) {
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

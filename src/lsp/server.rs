use super::protocol::{
    capabilities, health as send_health, info, log, publish, status as send_status, RESCAN,
};
use super::scan::{self, ScanResult};
use super::schedule::{mark_dirty, take_pending, Workspace};
use super::settings::Settings;
use super::workspace::{affects_scan, changed_paths, notification_path, roots as workspace_roots};
use anyhow::{Context, Result};
use crossbeam_channel::{unbounded, Sender};
use lsp_server::{Connection, Message, Request, Response};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub fn serve() -> Result<()> {
    let (connection, io_threads) = Connection::stdio();
    let (id, params) = connection
        .initialize_start()
        .context("waiting for initialize")?;
    let roots = workspace_roots(&params);
    let settings = Settings::from_lsp(&params);
    connection
        .initialize_finish(id, capabilities())
        .context("sending initialize result")?;
    let result = run(connection, roots, settings);
    // In-flight scans only write metrics/publish through the (now-closed)
    // sender; joining them keeps shutdown deterministic and lets bounded
    // flushes finish.
    scan::join_all();
    io_threads.join().context("joining LSP I/O threads")?;
    result
}

fn run(connection: Connection, roots: Vec<PathBuf>, mut settings: Settings) -> Result<()> {
    let (completed_tx, completed_rx) = unbounded();
    let mut workspaces = BTreeMap::new();
    for root in roots {
        queue(
            &mut workspaces,
            root,
            settings,
            &connection.sender,
            &completed_tx,
        )?;
    }
    launch_pending(settings, &connection.sender, &mut workspaces, &completed_tx)?;
    loop {
        crossbeam_channel::select! {
            recv(connection.receiver) -> message => match message {
                Ok(message) => if handle_message(message, &connection, &mut settings, &mut workspaces, &completed_tx)? { break; },
                Err(_) => break,
            },
            recv(completed_rx) -> result => if let Ok(result) = result {
                apply_result(result, &connection.sender, &mut workspaces)?;
                launch_pending(settings, &connection.sender, &mut workspaces, &completed_tx)?;
            },
        }
    }
    Ok(())
}

fn handle_message(
    message: Message,
    connection: &Connection,
    settings: &mut Settings,
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
    completed: &Sender<ScanResult>,
) -> Result<bool> {
    match message {
        Message::Request(request) => {
            handle_request(request, connection, *settings, workspaces, completed)
        }
        Message::Notification(notification) => {
            if notification.method == "exit" {
                return Ok(true);
            }
            if notification.method == "workspace/didChangeConfiguration" {
                *settings = Settings::from_lsp(&notification.params);
                rescan_all(workspaces, *settings, &connection.sender, completed)?;
            } else if notification.method == "textDocument/didSave" {
                let saved: Vec<_> = notification_path(&notification.params)
                    .into_iter()
                    .collect();
                rescan_affected(&saved, workspaces, *settings, &connection.sender, completed)?;
            } else if notification.method == "workspace/didChangeWatchedFiles" {
                let changed = changed_paths(&notification.params);
                rescan_affected(
                    &changed,
                    workspaces,
                    *settings,
                    &connection.sender,
                    completed,
                )?;
            }
            Ok(false)
        }
        Message::Response(_) => Ok(false),
    }
}

/// Rescan each workspace holding a changed file that can alter its scan.
fn rescan_affected(
    paths: &[PathBuf],
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
    settings: Settings,
    sender: &Sender<Message>,
    completed: &Sender<ScanResult>,
) -> Result<()> {
    let roots: BTreeSet<_> = paths
        .iter()
        .filter(|path| affects_scan(path))
        .filter_map(|path| root_for_path(path, workspaces))
        .collect();
    for root in roots {
        queue(workspaces, root, settings, sender, completed)?;
    }
    Ok(())
}

fn root_for_path(path: &Path, workspaces: &BTreeMap<PathBuf, Workspace>) -> Option<PathBuf> {
    workspaces
        .keys()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned()
}

fn handle_request(
    request: Request,
    connection: &Connection,
    settings: Settings,
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
    completed: &Sender<ScanResult>,
) -> Result<bool> {
    if connection.handle_shutdown(&request)? {
        return Ok(true);
    }
    if request.method == "workspace/executeCommand"
        && request.params.get("command").and_then(Value::as_str) == Some(RESCAN)
    {
        rescan_all(workspaces, settings, &connection.sender, completed)?;
        connection
            .sender
            .send(Message::Response(Response::new_ok(request.id, Value::Null)))
            .context("responding to rescan")?;
    } else {
        connection
            .sender
            .send(Message::Response(Response::new_err(
                request.id,
                -32601,
                "Unsupported request".to_owned(),
            )))
            .context("responding to unsupported request")?;
    }
    Ok(false)
}

fn queue(
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
    root: PathBuf,
    settings: Settings,
    sender: &Sender<Message>,
    completed: &Sender<ScanResult>,
) -> Result<()> {
    let Some(generation) = mark_dirty(workspaces, &root) else {
        return Ok(());
    };
    send_status(sender, &root, "scanning")?;
    scan::spawn(root, generation, settings, completed.clone());
    Ok(())
}

fn rescan_all(
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
    settings: Settings,
    sender: &Sender<Message>,
    completed: &Sender<ScanResult>,
) -> Result<()> {
    let roots: Vec<_> = workspaces.keys().cloned().collect();
    for root in roots {
        queue(workspaces, root, settings, sender, completed)?;
    }
    Ok(())
}

fn launch_pending(
    settings: Settings,
    sender: &Sender<Message>,
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
    completed: &Sender<ScanResult>,
) -> Result<()> {
    while let Some((path, generation)) = take_pending(workspaces) {
        send_status(sender, &path, "scanning")?;
        scan::spawn(path, generation, settings, completed.clone());
    }
    Ok(())
}

fn apply_result(
    result: ScanResult,
    sender: &Sender<Message>,
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
) -> Result<()> {
    let Some(workspace) = workspaces.get_mut(&result.root) else {
        return Ok(());
    };
    workspace.running = false;
    if result.generation != workspace.generation {
        return Ok(());
    }
    if let Some(notice) = result.notice {
        info(sender, notice)?;
    }
    if let Some(error) = result.error {
        if let Some(health) = result.health {
            send_health(sender, &health)?;
        }
        send_status(sender, &result.root, "error")?;
        return log(
            sender,
            format!("Sensez scan failed for {}: {error}", result.root.display()),
        );
    }
    let next: BTreeSet<_> = result.diagnostics.keys().cloned().collect();
    for uri in workspace.published.difference(&next) {
        publish(sender, uri, Vec::new())?;
    }
    for (uri, diagnostics) in result.diagnostics {
        publish(sender, &uri, diagnostics)?;
    }
    workspace.published = next;
    if let Some(health) = result.health {
        send_health(sender, &health)?;
    }
    send_status(sender, &result.root, "idle")
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;

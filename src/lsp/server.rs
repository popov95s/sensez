use super::diagnostics::{from_report, retain_visible};
use super::health::HealthSummary;
use super::protocol::{
    capabilities, health as send_health, log, publish, status as send_status, RESCAN,
};
use super::settings::{AnalysisScope, Settings};
use super::workspace::{notification_path, roots as workspace_roots};
use anyhow::{Context, Result};
use crossbeam_channel::{unbounded, Sender};
use lsp_server::{Connection, Message, Request, Response};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::thread;

pub(super) struct Workspace {
    path: PathBuf,
    generation: u64,
    running: bool,
    pending: bool,
    published: BTreeSet<String>,
}

struct ScanResult {
    root: PathBuf,
    generation: u64,
    diagnostics: BTreeMap<String, Vec<super::diagnostics::Diagnostic>>,
    health: Option<HealthSummary>,
    error: Option<String>,
}

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
    loop {
        crossbeam_channel::select! {
            recv(connection.receiver) -> message => match message {
                Ok(message) => if handle_message(message, &connection, &mut settings, &mut workspaces, &completed_tx)? { break; },
                Err(_) => break,
            },
            recv(completed_rx) -> result => if let Ok(result) = result {
                apply_result(result, settings, &connection.sender, &completed_tx, &mut workspaces)?;
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
                if let Some(root) = root_for_notification(&notification.params, workspaces) {
                    queue(workspaces, root, *settings, &connection.sender, completed)?;
                } else {
                    rescan_all(workspaces, *settings, &connection.sender, completed)?;
                }
            }
            Ok(false)
        }
        Message::Response(_) => Ok(false),
    }
}

fn root_for_notification(
    params: &Value,
    workspaces: &BTreeMap<PathBuf, Workspace>,
) -> Option<PathBuf> {
    let path = notification_path(params)?;
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
    let workspace = workspaces.entry(root.clone()).or_insert(Workspace {
        path: root.clone(),
        generation: 0,
        running: false,
        pending: false,
        published: BTreeSet::new(),
    });
    workspace.generation = workspace.generation.saturating_add(1);
    if workspace.running {
        workspace.pending = true;
        return Ok(());
    }
    workspace.running = true;
    send_status(sender, &root, "scanning")?;
    spawn_scan(root, workspace.generation, settings, completed.clone());
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

fn spawn_scan(root: PathBuf, generation: u64, settings: Settings, completed: Sender<ScanResult>) {
    thread::spawn(move || {
        let result = match crate::analyze_path(&root, None) {
            Ok((mut report, module_files)) => {
                let mut health = settings.health_enabled.then(|| {
                    HealthSummary::from_report(root.display().to_string(), "workspace", &report)
                });
                if settings.scope == AnalysisScope::Changed {
                    match crate::diff::git::changed_vs_head(&root) {
                        Ok(changed) => crate::diff::apply(&mut report, &changed, &module_files),
                        Err(error) => {
                            return send_scan_error(root, generation, health, error, completed)
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
                }
            }
            Err(error) => ScanResult {
                health: None,
                root,
                generation,
                diagnostics: BTreeMap::new(),
                error: Some(format!("{error:#}")),
            },
        };
        let _ = completed.send(result);
    });
}

fn send_scan_error(
    root: PathBuf,
    generation: u64,
    health: Option<HealthSummary>,
    error: anyhow::Error,
    completed: Sender<ScanResult>,
) {
    let _ = completed.send(ScanResult {
        root,
        generation,
        diagnostics: BTreeMap::new(),
        health,
        error: Some(format!("{error:#}")),
    });
}

fn apply_result(
    result: ScanResult,
    settings: Settings,
    sender: &Sender<Message>,
    completed: &Sender<ScanResult>,
    workspaces: &mut BTreeMap<PathBuf, Workspace>,
) -> Result<()> {
    let Some(workspace) = workspaces.get_mut(&result.root) else {
        return Ok(());
    };
    workspace.running = false;
    if result.generation != workspace.generation {
        if workspace.pending {
            workspace.pending = false;
            workspace.running = true;
            spawn_scan(
                workspace.path.clone(),
                workspace.generation,
                settings,
                completed.clone(),
            );
        }
        return Ok(());
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

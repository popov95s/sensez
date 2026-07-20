use super::diagnostics::{Diagnostic, PublishDiagnostics};
use super::health::HealthSummary;
use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use lsp_server::{Message, Notification};
use serde_json::{json, Value};
use std::path::Path;
use url::Url;

pub const RESCAN: &str = "sensez.rescan";
pub const STATUS: &str = "sensez/status";
const HEALTH: &str = "sensez/health";

pub fn publish(sender: &Sender<Message>, uri: &str, diagnostics: Vec<Diagnostic>) -> Result<()> {
    let parsed_uri = Url::parse(uri).context("parsing diagnostic file URI")?;
    send(
        sender,
        "textDocument/publishDiagnostics",
        &PublishDiagnostics {
            uri: parsed_uri,
            diagnostics,
        },
    )
}

pub fn status(sender: &Sender<Message>, root: &Path, state: &str) -> Result<()> {
    send(
        sender,
        STATUS,
        &json!({ "root": root.display().to_string(), "state": state }),
    )
}

pub fn health(sender: &Sender<Message>, value: &HealthSummary) -> Result<()> {
    send(sender, HEALTH, value)
}

pub fn log(sender: &Sender<Message>, message: String) -> Result<()> {
    send(
        sender,
        "window/logMessage",
        &json!({ "type": 1, "message": message }),
    )
}

pub fn capabilities() -> Value {
    json!({ "capabilities": { "textDocumentSync": { "openClose": true, "change": 0, "save": { "includeText": false } }, "executeCommandProvider": { "commands": [RESCAN] } } })
}

fn send(sender: &Sender<Message>, method: &str, params: &impl serde::Serialize) -> Result<()> {
    sender
        .send(Message::Notification(Notification::new(
            method.to_owned(),
            serde_json::to_value(params)?,
        )))
        .context("sending LSP notification")
}

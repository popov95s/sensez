use super::*;
use crossbeam_channel::Receiver;
use lsp_server::Notification;
use std::path::Path;

const ROOT: &str = "/work/a";

fn notifications(receiver: &Receiver<Message>) -> Vec<Notification> {
    receiver
        .try_iter()
        .filter_map(|message| match message {
            Message::Notification(notification) => Some(notification),
            _ => None,
        })
        .collect()
}

fn result(generation: u64, uris: &[&str]) -> ScanResult {
    ScanResult {
        root: PathBuf::from(ROOT),
        generation,
        diagnostics: uris
            .iter()
            .map(|uri| (uri.to_string(), Vec::new()))
            .collect(),
        health: None,
        error: None,
    }
}

#[test]
fn stale_result_is_dropped_without_publishing() {
    let (sender, receiver) = unbounded();
    let mut workspaces = BTreeMap::new();
    mark_dirty(&mut workspaces, Path::new(ROOT), 0);
    mark_dirty(&mut workspaces, Path::new(ROOT), 1);

    apply_result(
        result(1, &["file:///work/a/x.py"]),
        &sender,
        &mut workspaces,
    )
    .unwrap();

    assert!(notifications(&receiver).is_empty());
    assert!(!workspaces[Path::new(ROOT)].running);
}

#[test]
fn files_without_findings_any_more_get_their_diagnostics_cleared() {
    let (sender, receiver) = unbounded();
    let mut workspaces = BTreeMap::new();
    mark_dirty(&mut workspaces, Path::new(ROOT), 0);
    let both = result(1, &["file:///work/a/x.py", "file:///work/a/y.py"]);
    apply_result(both, &sender, &mut workspaces).unwrap();
    mark_dirty(&mut workspaces, Path::new(ROOT), 0);
    notifications(&receiver);

    apply_result(
        result(2, &["file:///work/a/y.py"]),
        &sender,
        &mut workspaces,
    )
    .unwrap();

    let published: Vec<_> = notifications(&receiver)
        .into_iter()
        .filter(|n| n.method == "textDocument/publishDiagnostics")
        .map(|n| n.params["uri"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(published, ["file:///work/a/x.py", "file:///work/a/y.py"]);
    assert_eq!(
        workspaces[Path::new(ROOT)].published,
        BTreeSet::from(["file:///work/a/y.py".to_string()])
    );
}

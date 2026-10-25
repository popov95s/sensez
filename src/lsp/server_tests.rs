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
        notice: None,
    }
}

#[test]
fn stale_result_is_dropped_without_publishing() {
    let (sender, receiver) = unbounded();
    let mut workspaces = BTreeMap::new();
    mark_dirty(&mut workspaces, Path::new(ROOT));
    mark_dirty(&mut workspaces, Path::new(ROOT));

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
    mark_dirty(&mut workspaces, Path::new(ROOT));
    let both = result(1, &["file:///work/a/x.py", "file:///work/a/y.py"]);
    apply_result(both, &sender, &mut workspaces).unwrap();
    mark_dirty(&mut workspaces, Path::new(ROOT));
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

#[test]
fn saving_a_non_source_file_does_not_rescan() {
    let (sender, _receiver) = unbounded();
    let (completed, _results) = unbounded();
    let mut workspaces = BTreeMap::new();
    mark_dirty(&mut workspaces, Path::new(ROOT));
    workspaces.get_mut(Path::new(ROOT)).unwrap().running = false;

    let saved = [
        PathBuf::from("/work/a/README.md"),
        PathBuf::from("/work/a/yarn.lock"),
    ];
    rescan_affected(
        &saved,
        &mut workspaces,
        Settings::default(),
        &sender,
        &completed,
    )
    .unwrap();

    assert_eq!(workspaces[Path::new(ROOT)].generation, 1);
}

#[test]
fn changed_scope_outside_git_scans_the_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    crate::test_support::write(tmp.path(), "app.py", "def run():\n    return 1\n");
    let settings = Settings {
        health_enabled: false,
        ..Settings::default()
    };

    let result = super::scan::scan(tmp.path().to_path_buf(), 1, settings);

    assert_eq!(result.error, None);
    assert!(result.notice.is_some());
}

use super::*;

fn root(name: &str) -> PathBuf {
    PathBuf::from(format!("/work/{name}"))
}

#[test]
fn idle_workspace_with_capacity_starts_a_scan() {
    let mut workspaces = BTreeMap::new();
    assert_eq!(mark_dirty(&mut workspaces, &root("a")), Some(1));
    assert!(workspaces[&root("a")].running);
}

#[test]
fn running_workspace_is_marked_pending_with_a_new_generation() {
    let mut workspaces = BTreeMap::new();
    mark_dirty(&mut workspaces, &root("a"));
    assert_eq!(mark_dirty(&mut workspaces, &root("a")), None);
    let workspace = &workspaces[&root("a")];
    assert!(workspace.pending);
    assert_eq!(workspace.generation, 2);
}

#[test]
fn pending_workspace_starts_as_soon_as_a_running_scan_finishes() {
    let mut workspaces = BTreeMap::new();
    mark_dirty(&mut workspaces, &root("a"));
    mark_dirty(&mut workspaces, &root("b"));
    assert_eq!(mark_dirty(&mut workspaces, &root("c")), None);
    assert_eq!(take_pending(&mut workspaces), None);

    workspaces.get_mut(&root("a")).unwrap().running = false;

    assert_eq!(take_pending(&mut workspaces), Some((root("c"), 1)));
    assert!(!workspaces[&root("c")].pending);
}

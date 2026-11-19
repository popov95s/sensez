use super::{AnalysisSession, MAX_WORKSPACES};

#[test]
fn reuses_unchanged_files_and_reparses_a_changed_file() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("module.py");
    std::fs::write(&file, "def first():\n    return 1\n").unwrap();
    let session = AnalysisSession::default();
    let first = crate::spine::cache::load_project(std::slice::from_ref(&file)).unwrap();
    assert_eq!(session.parse(root.path(), &first).1.reusable, 0);
    assert_eq!(session.parse(root.path(), &first).1.reusable, 1);

    std::fs::write(&file, "def second():\n    return 2\n").unwrap();
    let changed = crate::spine::cache::load_project(&[file]).unwrap();
    let stats = session.parse(root.path(), &changed).1;
    assert_eq!((stats.modified, stats.reusable), (1, 0));
}

#[test]
fn a_parse_issue_in_one_file_keeps_the_others_cached() {
    let root = tempfile::tempdir().unwrap();
    let good = root.path().join("good.py");
    let deep = root.path().join("deep.py");
    std::fs::write(&good, "def ok():\n    return 1\n").unwrap();
    std::fs::write(
        &deep,
        format!("x = {}1{}\n", "[".repeat(600), "]".repeat(600)),
    )
    .unwrap();
    let session = AnalysisSession::default();
    let project = crate::spine::cache::load_project(&[good, deep]).unwrap();

    let (parsed, _) = session.parse(root.path(), &project);
    assert_eq!(parsed.issues.len(), 1);

    assert_eq!(session.parse(root.path(), &project).1.reusable, 1);
}

#[test]
fn concurrent_workspaces_parse_independently() {
    let session = AnalysisSession::default();
    let roots: Vec<_> = (0..4)
        .map(|i| {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(
                dir.path().join(format!("m{i}.py")),
                format!("def f{i}():\n    return {i}\n"),
            )
            .unwrap();
            dir
        })
        .collect();
    let projects: Vec<_> = roots
        .iter()
        .map(|dir| {
            let files: Vec<_> = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            crate::spine::cache::load_project(&files).unwrap()
        })
        .collect();
    std::thread::scope(|scope| {
        for (root, project) in roots.iter().zip(&projects) {
            scope.spawn(|| {
                let (parsed, _) = session.parse(root.path(), project);
                assert!(parsed.issues.is_empty());
            });
        }
    });
    for root in &roots {
        assert!(session.has_workspace(root.path()));
    }
}

/// Eviction follows least-recent use, not arbitrary map order.
#[test]
fn evicts_least_recently_used_workspace() {
    use std::thread::sleep;
    use std::time::Duration;

    let session = AnalysisSession::default();
    let dirs: Vec<_> = (0..MAX_WORKSPACES as u8 + 1)
        .map(|i| {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(
                dir.path().join(format!("m{i}.py")),
                format!("def f{i}():\n    return {i}\n"),
            )
            .unwrap();
            dir
        })
        .collect();
    let project_for = |dir: &tempfile::TempDir| {
        let files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        crate::spine::cache::load_project(&files).unwrap()
    };

    for dir in &dirs[..usize::from(MAX_WORKSPACES as u8)] {
        session.parse(dir.path(), &project_for(dir));
        sleep(Duration::from_millis(5));
    }
    session.parse(dirs[0].path(), &project_for(&dirs[0]));
    sleep(Duration::from_millis(5));

    let newest = dirs.last().unwrap();
    session.parse(newest.path(), &project_for(newest));

    assert!(
        session.has_workspace(dirs[0].path()),
        "recently touched stays"
    );
    assert!(
        !session.has_workspace(dirs[1].path()),
        "untouched workspace is evicted"
    );
    assert!(session.has_workspace(newest.path()));
}

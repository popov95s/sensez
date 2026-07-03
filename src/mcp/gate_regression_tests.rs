use super::gate::gate;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn gate_blocks_only_new_finding_identities_after_prior_block() {
    let Some(repo) = fresh_repo("work") else {
        return;
    };
    std::fs::create_dir_all(&repo.dir).unwrap();
    std::fs::write(repo.dir.join("__init__.py"), "").unwrap();
    let left = repo.dir.join("left.py");
    let right = repo.dir.join("right.py");
    std::fs::write(
        &left,
        "def live_left():\n    return 0\n\n\ndef alpha():\n    return 1\n",
    )
    .unwrap();
    std::fs::write(
        &right,
        "def live_right():\n    return 0\n\n\ndef steady():\n    return 2\n",
    )
    .unwrap();

    let first = block_reason(&gate(&json!({"path": repo.path})).unwrap());
    assert!(
        first.contains("2 diff finding(s)"),
        "first block should include both findings: {first}"
    );

    std::fs::write(
        &left,
        "def live_left():\n    return 0\n\n\ndef alpha():\n    return 1\n\n\ndef fresh():\n    return 3\n",
    )
    .unwrap();

    let second = block_reason(&gate(&json!({"path": repo.path})).unwrap());
    assert!(
        second.contains("1 diff finding(s)"),
        "second block should include only the new identity: {second}"
    );
}

#[test]
fn gate_keeps_prior_block_memory_when_head_is_detached() {
    let Some(repo) = fresh_repo("work") else {
        return;
    };
    std::fs::create_dir_all(&repo.dir).unwrap();
    std::fs::write(repo.dir.join("__init__.py"), "").unwrap();
    std::fs::write(
        repo.dir.join("left.py"),
        "def live_left():\n    return 0\n\n\ndef alpha():\n    return 1\n",
    )
    .unwrap();

    let first = gate(&json!({"path": repo.path})).unwrap();
    assert_block(&first);
    let second = gate(&json!({"path": repo.path})).unwrap();
    assert_allow(&second);

    assert!(git(&repo.root, &["checkout", "--detach"]));
    let detached = gate(&json!({"path": repo.path})).unwrap();
    assert_allow(&detached);

    assert!(git(&repo.root, &["checkout", "master"]) || git(&repo.root, &["checkout", "main"]));
    let attached = gate(&json!({"path": repo.path})).unwrap();
    assert_allow(&attached);
}

fn block_reason(resp: &Value) -> String {
    let text = resp["content"][0]["text"].as_str().unwrap();
    let decision: Value = serde_json::from_str(text).unwrap();
    assert_eq!(decision["decision"], "block", "expected block: {text}");
    decision["reason"].as_str().unwrap().to_string()
}

fn assert_block(resp: &Value) {
    let text = resp["content"][0]["text"].as_str().unwrap();
    let decision: Value = serde_json::from_str(text).unwrap();
    assert_eq!(decision["decision"], "block", "expected block: {text}");
}

fn assert_allow(resp: &Value) {
    assert_eq!(resp["content"][0]["text"], "{}");
}

struct TestRepo {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    dir: PathBuf,
    path: String,
}

fn fresh_repo(child: &str) -> Option<TestRepo> {
    let tmp = tempfile::tempdir().ok()?;
    let root = tmp.path().to_path_buf();
    if !init_repo(&root) {
        return None;
    }
    Some(TestRepo {
        _tmp: tmp,
        root: root.clone(),
        dir: root.join(child),
        path: root.to_string_lossy().into_owned(),
    })
}

fn init_repo(root: &Path) -> bool {
    if !git(root, &["init"]) {
        return false;
    }
    std::fs::write(
        root.join("base.py"),
        "from work.left import live_left\nfrom work.right import live_right\n\nprint(live_left(), live_right())\n",
    )
    .unwrap();
    git(root, &["add", "."])
        && git(
            root,
            &[
                "-c",
                "user.email=sensez@example.test",
                "-c",
                "user.name=Sensez",
                "commit",
                "-m",
                "base",
            ],
        )
}

fn git(root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

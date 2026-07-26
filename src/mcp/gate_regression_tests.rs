use super::gate::gate;
use crate::test_support::GitTestRepo;
use serde_json::{json, Value};

#[test]
fn gate_blocks_only_new_finding_identities_after_prior_block() {
    let Some(repo) = fresh_repo("work") else {
        return;
    };
    std::fs::create_dir_all(&repo.file).unwrap();
    std::fs::write(repo.file.join("__init__.py"), "").unwrap();
    let left = repo.file.join("left.py");
    let right = repo.file.join("right.py");
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
    std::fs::create_dir_all(&repo.file).unwrap();
    std::fs::write(repo.file.join("__init__.py"), "").unwrap();
    std::fs::write(
        repo.file.join("left.py"),
        "def live_left():\n    return 0\n\n\ndef alpha():\n    return 1\n",
    )
    .unwrap();

    let first = gate(&json!({"path": repo.path})).unwrap();
    assert_block(&first);
    let second = gate(&json!({"path": repo.path})).unwrap();
    assert_allow(&second);

    assert!(repo.git(&["checkout", "--detach"]));
    let detached = gate(&json!({"path": repo.path})).unwrap();
    assert_allow(&detached);

    assert!(repo.git(&["checkout", "master"]) || repo.git(&["checkout", "main"]));
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

fn fresh_repo(child: &str) -> Option<GitTestRepo> {
    GitTestRepo::new(
        child,
        "from work.left import live_left\nfrom work.right import live_right\n\nprint(live_left(), live_right())\n",
    )
}

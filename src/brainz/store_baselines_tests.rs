use super::*;
use crate::brainz::fingerprint::{AgedEntry, Detector, Label, ResolvedRecord};
use crate::report::Pillar;

fn prints(symbol: &str, first_seen: u64) -> Aged {
    BTreeMap::from([(
        Pillar::DeadCode,
        BTreeMap::from([(
            format!("{symbol}-key"),
            AgedEntry {
                first_seen,
                label: Label::DeadCode {
                    module: "app".to_string(),
                    symbol: symbol.to_string(),
                    symbol_kind: "function".to_string(),
                },
                class: Detector::DeadCode {
                    symbol_kind: "function".to_string(),
                },
            },
        )]),
    )])
}

fn history(key: &str) -> ResolvedHistory {
    BTreeMap::from([(
        key.to_string(),
        ResolvedRecord {
            class: Detector::DeadCode {
                symbol_kind: "function".to_string(),
            },
            resolved_ts: 7,
        },
    )])
}

/// The single-file layout every earlier release wrote.
fn write_legacy(root: &Path, branches: &[(&str, u64, Aged, ResolvedHistory)]) {
    let entries: serde_json::Map<String, serde_json::Value> = branches
        .iter()
        .map(|(branch, updated, prints, history)| {
            let entry =
                serde_json::json!({"updated": updated, "prints": prints, "history": history});
            (branch.to_string(), entry)
        })
        .collect();
    std::fs::create_dir_all(dir(root)).unwrap();
    let legacy = serde_json::json!({ "branches": entries });
    std::fs::write(legacy_path(root), serde_json::to_vec(&legacy).unwrap()).unwrap();
}

fn snapshot(root: &Path, branch: &str) -> (Aged, ResolvedHistory, Option<u64>, bool) {
    (
        load_fingerprints(root, branch),
        load_resolved_history(root, branch),
        branch_updated(root, branch),
        branch_state(root, branch).1,
    )
}

#[test]
fn legacy_single_file_baselines_read_the_same_before_and_after_migrating() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_legacy(
        root,
        &[
            ("main", 5, prints("alpha", 1), history("gone")),
            ("feature/x", 6, prints("beta", 2), ResolvedHistory::new()),
        ],
    );
    let before = (snapshot(root, "main"), snapshot(root, "feature/x"));

    update_fingerprints(root, "other", 9, |_, _| (prints("gamma", 3), history("h"))).unwrap();

    assert_eq!(
        (snapshot(root, "main"), snapshot(root, "feature/x")),
        before
    );
    assert!(branch_path(root, "main").exists());
    assert!(!legacy_path(root).exists());
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(index_path(root)).unwrap()).unwrap();
    assert!(index["branches"]["main"].get("prints").is_none());
    assert_eq!(snapshot(root, "other").0, prints("gamma", 3));
}

#[test]
fn unchanged_fingerprints_keep_the_branch_file_and_restamp_the_index() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    save_fingerprints(root, "main", &prints("alpha", 1), &history("h"), 1).unwrap();
    let file = branch_path(root, "main");
    let pretty = serde_json::to_vec_pretty(&BranchData {
        prints: prints("alpha", 1),
        history: history("h"),
    })
    .unwrap();
    std::fs::write(&file, &pretty).unwrap();

    save_fingerprints(root, "main", &prints("alpha", 1), &history("h"), 2).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), pretty);
    assert_eq!(branch_updated(root, "main"), Some(2));

    save_fingerprints(root, "main", &prints("beta", 1), &history("h"), 3).unwrap();
    assert_ne!(std::fs::read(&file).unwrap(), pretty);
    assert_eq!(load_fingerprints(root, "main"), prints("beta", 1));
}

#[test]
fn pruned_branches_lose_their_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    for n in 0..=MAX_BRANCHES as u64 {
        let branch = format!("b{n}");
        save_fingerprints(
            root,
            &branch,
            &prints("alpha", n),
            &ResolvedHistory::new(),
            n,
        )
        .unwrap();
    }

    assert_eq!(branch_updated(root, "b0"), None);
    assert!(!branch_path(root, "b0").exists());
    assert!(branch_path(root, "b1").exists());
}

#[test]
fn stale_expected_update_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    save_fingerprints(
        root,
        "main",
        &prints("alpha", 1),
        &ResolvedHistory::new(),
        5,
    )
    .unwrap();
    let write = BranchWrite {
        branch: "main",
        prints: prints("beta", 1),
        history: ResolvedHistory::new(),
        now: 6,
    };

    assert!(!save_fingerprints_if_current(root, write, 4).unwrap());
    assert_eq!(load_fingerprints(root, "main"), prints("alpha", 1));
    assert_eq!(branch_updated(root, "main"), Some(5));
}

use super::*;

fn env(cache_dir: &Path, offline: bool) -> ModelEnv {
    ModelEnv {
        model_dir: None,
        cache: Cache::new(cache_dir.to_path_buf()),
        offline,
    }
}

fn warm(cache_dir: &Path) -> PathBuf {
    Cache::new(cache_dir.to_path_buf())
        .repo(model_repo())
        .create_ref(MODEL_REVISION)
        .unwrap();
    warm_without_ref(cache_dir)
}

/// The layout Python `huggingface_hub` leaves for a commit revision: snapshot files, no ref.
fn warm_without_ref(cache_dir: &Path) -> PathBuf {
    let snapshot = cache_dir
        .join(format!("models--{}", MODEL_ID.replace('/', "--")))
        .join("snapshots")
        .join(MODEL_REVISION);
    std::fs::create_dir_all(&snapshot).unwrap();
    for file in MODEL_FILES {
        std::fs::write(snapshot.join(file), b"x").unwrap();
    }
    snapshot
}

#[test]
fn explicit_model_dir_wins() {
    let tmp = tempfile::tempdir().unwrap();
    let mut model_env = env(tmp.path(), true);
    model_env.model_dir = Some(tmp.path().join("copied"));

    assert_eq!(
        resolve(&model_env).unwrap(),
        Source::Local(tmp.path().join("copied"))
    );
}

#[test]
fn warm_cache_is_used_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let snapshot = warm(tmp.path());

    assert_eq!(
        resolve(&env(tmp.path(), true)).unwrap(),
        Source::Local(snapshot)
    );
}

#[test]
fn hub_warmed_sha_snapshot_without_ref_is_used_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let snapshot = warm_without_ref(tmp.path());

    assert_eq!(
        resolve(&env(tmp.path(), true)).unwrap(),
        Source::Local(snapshot)
    );
}

#[test]
fn only_a_forty_hex_revision_is_a_commit_sha() {
    assert!(is_commit_sha(MODEL_REVISION));
    assert!(!is_commit_sha("main"));
    assert!(!is_commit_sha(&MODEL_REVISION[1..]));
}

#[test]
fn empty_cache_offline_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();

    let error = resolve(&env(tmp.path(), true)).unwrap_err().to_string();

    assert!(error.contains("offline mode is set"), "{error}");
    assert!(error.contains("SENSEZ_EYEZ_MODEL_DIR"), "{error}");
}

#[test]
fn empty_cache_online_plans_a_download() {
    let tmp = tempfile::tempdir().unwrap();

    assert_eq!(resolve(&env(tmp.path(), false)).unwrap(), Source::Download);
}

#[test]
fn download_notice_is_one_line_naming_the_overrides() {
    let notice = download_notice(Path::new("/cache/hub"));

    assert!(!notice.contains('\n'));
    assert!(notice.contains("/cache/hub"));
    assert!(notice.contains("HF_HOME"));
    assert!(notice.contains("SENSEZ_EYEZ_MODEL_DIR"));
}

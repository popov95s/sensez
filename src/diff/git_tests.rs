use super::*;
use std::fs;

#[test]
fn untracked_directory_is_expanded_to_source_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    if Command::new("git")
        .arg("init")
        .current_dir(root)
        .output()
        .is_err()
    {
        return;
    }
    let pkg = root.join("newpkg/src/deep");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(pkg.join("a.py"), "def f():\n    pass\n").unwrap();
    fs::write(pkg.join("b.ts"), "export const x = 1;\n").unwrap();
    fs::write(pkg.join("notes.md"), "# notes\n").unwrap();

    let found = untracked_sources(root).unwrap();
    let names: Vec<String> = found
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.contains(&"a.py".to_string()),
        "nested .py expanded: {names:?}"
    );
    assert!(
        !names.contains(&"notes.md".to_string()),
        "non-source excluded"
    );
    #[cfg(feature = "lang-typescript")]
    assert!(
        names.contains(&"b.ts".to_string()),
        "untracked .ts included: {names:?}"
    );
}

#[test]
fn diff_is_fast_with_large_gitignored_footprint() {
    use std::time::Instant;

    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git must be available")
    };
    git(&["init"]);
    git(&["config", "user.email", "test@test"]);
    git(&["config", "user.name", "test"]);

    fs::write(root.join(".gitignore"), ".venv/\nnode_modules/\n").unwrap();
    fs::write(root.join("seed.py"), "# seed\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-m", "seed"]);

    let venv1 = root.join(".venv/lib/python3.11/site-packages");
    let venv2 = root.join("node_modules/pkg/dist");
    fs::create_dir_all(&venv1).unwrap();
    fs::create_dir_all(&venv2).unwrap();
    fs::create_dir_all(&venv1).unwrap();
    fs::create_dir_all(&venv2).unwrap();
    for i in 0..500 {
        fs::write(venv1.join(format!("mod{i}.py")), format!("# {i}\n")).unwrap();
        fs::write(venv2.join(format!("chunk{i}.js")), format!("// {i}\n")).unwrap();
    }
    fs::write(root.join("app.py"), "def main():\n    return 42\n").unwrap();

    let start = Instant::now();
    let changed = changed_vs_head(root).unwrap();
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 2,
        "diff must complete in < 2 s, took {elapsed:.2?}"
    );
    assert!(changed.touches_file(&root.join("app.py")));
    assert!(!changed.paths().any(|p| p.starts_with(&venv1)));
    assert!(!changed.paths().any(|p| p.starts_with(&venv2)));
}

#[test]
fn large_output_does_not_deadlock_or_truncate() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c").arg("head -c 200000 /dev/zero | tr '\\0' 'x'");
    let bytes = run_with_timeout(&mut cmd, tmp.path())
        .expect("200 KB of stdout must complete, not hit the timeout");
    assert!(bytes.status.success());
    assert_eq!(bytes.stdout.len(), 200_000);
}

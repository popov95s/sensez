use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn sensez(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sensez"))
        .args(["noze"])
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

fn project(config: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("demo.py"),
        "def add(a, b):\n    return a + b\n",
    )
    .unwrap();
    fs::write(tmp.path().join("sensez.toml"), config).unwrap();
    tmp
}

#[test]
fn clean_scan_exits_zero() {
    let tmp = project("");
    let out = sensez(tmp.path(), &["--fail-on", "must_fix"]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn tripped_gate_exits_one() {
    let tmp = project("[action]\ncycles = \"must_fix\"\n");
    fs::write(tmp.path().join("alpha.py"), "import beta\n").unwrap();
    fs::write(tmp.path().join("beta.py"), "import alpha\n").unwrap();
    let out = sensez(tmp.path(), &["--fail-on", "must_fix"]);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn invalid_config_warns_once_and_scans_on_defaults() {
    let tmp = project("[smells.rules.long_function]\nmax_line = 20\n");
    let out = sensez(tmp.path(), &["--json"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(!out.stdout.is_empty());
    assert_eq!(
        stderr
            .matches("sensez.toml:2:1: unknown key `smells.rules.long_function.max_line` (did you mean `max_lines`?)")
            .count(),
        1,
        "{stderr}"
    );
}

#[test]
fn out_of_range_semantic_score_warns_and_scans_on_defaults() {
    let tmp = project("[duplication.semantic]\nmin_shape_score = 101\n");
    let out = sensez(tmp.path(), &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains("min_shape_score must be at most 100"),
        "{stderr}"
    );
}

#[test]
fn config_warning_is_printed_once() {
    let tmp = project("[duplication]\ntreshold = 40\n");
    let out = sensez(tmp.path(), &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert_eq!(
        stderr.matches("duplication.treshold").count(),
        1,
        "{stderr}"
    );
}

#[test]
fn diff_outside_git_exits_two() {
    let tmp = project("");
    let out = sensez(tmp.path(), &["--diff"]);
    assert_eq!(out.status.code(), Some(2));
}

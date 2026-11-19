use super::*;

#[test]
fn prefix_args_pin_flags_per_runner_and_mode() {
    let cases = [
        (
            RunnerKind::Vitest,
            RunMode::Narrowed,
            vec!["run", "--passWithNoTests"],
        ),
        (RunnerKind::Vitest, RunMode::Full, vec!["run"]),
        (
            RunnerKind::Jest,
            RunMode::Narrowed,
            vec!["--runTestsByPath", "--passWithNoTests"],
        ),
        (RunnerKind::Jest, RunMode::Full, vec![]),
        (RunnerKind::Pytest, RunMode::Narrowed, vec![]),
        (RunnerKind::Pytest, RunMode::Full, vec![]),
    ];
    for (kind, mode, expected) in cases {
        assert_eq!(prefix_args(kind, mode), expected, "{kind:?} {mode:?}");
    }
}

#[test]
fn pytest_no_tests_exit_passes_only_when_narrowed() {
    let plan = |kind, mode| RunnerPlan {
        kind,
        mode,
        root: PathBuf::new(),
        program: PathBuf::new(),
        prefix_args: Vec::new(),
        tests: Vec::new(),
    };
    assert!(is_empty_narrowed_pytest(
        &plan(RunnerKind::Pytest, RunMode::Narrowed),
        Some(5)
    ));
    assert!(!is_empty_narrowed_pytest(
        &plan(RunnerKind::Pytest, RunMode::Full),
        Some(5)
    ));
    assert!(!is_empty_narrowed_pytest(
        &plan(RunnerKind::Vitest, RunMode::Narrowed),
        Some(5)
    ));
}

from .reports import ReportFields, detector_count, required_map


def assert_exact_transition_count(
    report: object,
    detector: str,
    target_name: str,
    *,
    resolved: int,
    reintroduced: int,
) -> None:
    actual_resolved = detector_count(report, "resolved_by_detector", detector)
    actual_reintroduced = detector_count(report, "reintroduced_by_detector", detector)
    if actual_resolved != resolved or actual_reintroduced != reintroduced:
        raise AssertionError(
            f"{target_name}: expected {detector} transitions "
            f"resolved={resolved}, reintroduced={reintroduced}; got "
            f"resolved={actual_resolved}, reintroduced={actual_reintroduced}"
        )


def assert_same_reported(
    report: object,
    expected: ReportFields,
    target_name: str,
    step: str,
) -> None:
    actual = required_map(
        report,
        ("all_time", "reported_by_detector"),
        target_name,
    )
    if actual != expected:
        raise AssertionError(
            f"{target_name}: reported_by_detector changed on {step}: "
            f"expected {expected.values!r}, got {actual.values!r}"
        )
